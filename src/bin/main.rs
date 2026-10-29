#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

//! Pilote l'i-Sobot depuis l'app Dabble (module GamePad) :
//! téléphone --BLE--> ESP32 --IR (GPIO18)--> i-Sobot.
//!
//! - Croix directionnelle : marcher (↑ ↓, diagonales) et tourner (← →), tant qu'elle est appuyée.
//! - Joystick : marcher dans 8 directions, pas chassés compris.
//! - △ ○ ✕ □ : actions de la banque courante (voir manette/banques.csv).
//! - Select : banque suivante, toutes catégories à la suite ; Start : catégorie suivante.
//!   Appui long (au moins `APPUI_LONG`) : banque ou catégorie précédente.
//!   L'écran OLED affiche la banque et ses 4 actions. Sans écran, la LED bleue fait un
//!   clignotement long par numéro de catégorie (quand elle change), puis un court par
//!   numéro de banque dans la catégorie.

use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::Async;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::rmt::{self, Rmt, Tx, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ble::controller::BleConnector;
use isobot::dabble::{self, Direction, Manette, Message, Parseur, bouton, fleche};
use isobot::ecran::{self, Vue};
use isobot::isobot::{self as ir, Canal, cmd};
use isobot::manette::{
    BANQUES, banque_precedente, banque_suivante, categorie_precedente, categorie_suivante,
    numero_categorie, position,
};
use log::{debug, info, warn};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use trouble_host::prelude::*;

extern crate alloc;

/// Nom visible depuis le téléphone.
const NOM: &str = "i-Sobot";
/// Canal réglé sur le robot (interrupteur A/B de la télécommande d'origine).
const CANAL: Canal = Canal::A;
/// Écart entre deux trames IR ; la marche est entretenue en répétant la trame à ce rythme.
const ECART_TRAMES: Duration = Duration::from_millis(200);
/// Durée à partir de laquelle Select ou Start reculent au lieu d'avancer.
const APPUI_LONG: Duration = Duration::from_millis(500);

/// Porteuse 38 kHz, rapport cyclique 1/3, comptée sur l'horloge source du RMT (80 MHz).
const RMT_SOURCE_HZ: u32 = 80_000_000;
const PORTEUSE_TICKS: u32 = RMT_SOURCE_HZ / 38_000;
const PORTEUSE_HAUT: u16 = (PORTEUSE_TICKS / 3) as u16;
const PORTEUSE_BAS: u16 = (PORTEUSE_TICKS - PORTEUSE_TICKS / 3) as u16;

/// Actions ponctuelles à émettre.
static ACTIONS: channel::Channel<CriticalSectionRawMutex, u32, 4> = channel::Channel::new();
/// Code de marche à répéter, `None` pour s'arrêter.
static MARCHE: Signal<CriticalSectionRawMutex, Option<u32>> = Signal::new();
/// Clignotements de la LED à faire : (longs, courts).
static CLIGNOTER: Signal<CriticalSectionRawMutex, (u8, u8)> = Signal::new();
/// Contenu à afficher sur l'écran OLED.
static ECRAN: Signal<CriticalSectionRawMutex, Vue> = Signal::new();

#[allow(
    clippy::large_stack_frames,
    reason = "table d'attributs construite une seule fois dans main"
)]
mod gatt {
    use trouble_host::prelude::*;

    /// Service « Nordic UART » attendu par Dabble.
    #[gatt_service(uuid = "6e400001-b5a3-f393-e0a9-e50e24dcca9e")]
    pub struct NordicUart {
        /// Téléphone → ESP32
        #[characteristic(uuid = "6e400002-b5a3-f393-e0a9-e50e24dcca9e", write, write_without_response, value = [0; 64])]
        pub rx: [u8; 64],
        /// ESP32 → téléphone
        #[characteristic(uuid = "6e400003-b5a3-f393-e0a9-e50e24dcca9e", notify)]
        pub tx: [u8; 20],
    }

    #[gatt_server]
    pub struct Server {
        pub uart: NordicUart,
    }
}
use gatt::Server;

/// UUID du service, octets dans l'ordre de l'annonce BLE (petit-boutiste).
const NUS_UUID: [u8; 16] = [
    0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0, 0x93, 0xf3, 0xa3, 0xb5, 0x01, 0x00, 0x40, 0x6e,
];

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 2.0.0
    // generator parameters: -o esp32 -o unstable-hal -o alloc -o embassy -o ble-trouble -o esp-backtrace -o log -o vscode

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // Émetteur IR : RMT à 1 µs par tick, porteuse sur les niveaux hauts.
    let rmt = Rmt::new(peripherals.RMT, Rate::from_hz(RMT_SOURCE_HZ))
        .unwrap()
        .into_async();
    let emetteur = rmt
        .channel0
        .configure_tx(
            &TxChannelConfig::default()
                .with_clk_divider((RMT_SOURCE_HZ / 1_000_000) as u8)
                .with_idle_output(true)
                .with_idle_output_level(Level::Low)
                .with_carrier_modulation(true)
                .with_carrier_high(PORTEUSE_HAUT)
                .with_carrier_low(PORTEUSE_BAS)
                .with_carrier_level(Level::High),
        )
        .unwrap()
        .with_pin(peripherals.GPIO18);
    spawner.spawn(emetteur_ir(emetteur).unwrap());

    // LED bleue de la carte
    let led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    spawner.spawn(led_task(led).unwrap());

    // Écran OLED SSD1306 128×64 en I2C : SDA = GPIO21, SCL = GPIO22
    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();
    spawner.spawn(ecran_task(i2c).unwrap());
    // L'initialisation du Bluetooth bloque l'exécuteur : on laisse d'abord l'écran
    // s'initialiser et afficher « Démarrage ».
    Timer::after_millis(150).await;

    let transport = BleConnector::new(peripherals.BT, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 20>::new(transport);
    let mut resources: HostResources<_, DefaultPacketPool, 1, 2> = HostResources::new();
    let stack = trouble_host::new(ble_controller, &mut resources)
        .set_random_address(Address::random([0x5b, 0x0b, 0x15, 0x0b, 0x07, 0xc5]))
        .build();
    let runner = stack.runner();
    let mut peripheral = stack.peripheral();
    let server = Server::new_with_config(GapConfig::default(NOM)).unwrap();

    info!("[ble] pile initialisée, annonce sous le nom « {NOM} »");
    join(ble_task(runner), async {
        let mut deconnecte = false;
        loop {
            match annoncer(&mut peripheral, &server, deconnecte).await {
                Ok(conn) => {
                    session(&server, &conn).await;
                    // Ne pas laisser le robot marcher si le téléphone se déconnecte.
                    MARCHE.signal(None);
                    deconnecte = true;
                }
                Err(e) => panic!("[ble] erreur d'annonce : {e:?}"),
            }
        }
    })
    .await;
    unreachable!()
}

async fn ble_task<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(e) = runner.run().await {
            panic!("[ble] erreur : {e:?}");
        }
    }
}

/// Annonce l'ESP32 et attend la connexion du téléphone.
/// `deconnecte` : le téléphone vient de se déconnecter (affiché sur l'écran).
#[allow(
    clippy::large_stack_frames,
    reason = "futur de la pile BLE, une seule instance à la fois"
)]
async fn annoncer<'v, 's, C: Controller>(
    peripheral: &mut Peripheral<'v, C, DefaultPacketPool>,
    server: &'s Server<'v>,
    deconnecte: bool,
) -> Result<GattConnection<'v, 's, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut adv_data = [0; 31];
    let adv_len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(NOM.as_bytes()),
        ],
        &mut adv_data[..],
    )?;
    let mut scan_data = [0; 31];
    let scan_len = AdStructure::encode_slice(
        &[AdStructure::CompleteServiceUuids128(&[NUS_UUID])],
        &mut scan_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &adv_data[..adv_len],
                scan_data: &scan_data[..scan_len],
            },
        )
        .await?;
    // Seulement maintenant l'ESP32 est visible depuis le téléphone.
    info!("[ble] en attente du téléphone…");
    ECRAN.signal(Vue::Attente { deconnecte });
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[ble] téléphone connecté");
    Ok(conn)
}

/// Traite les trames Dabble jusqu'à la déconnexion.
#[allow(
    clippy::large_stack_frames,
    reason = "futur de la pile BLE, une seule instance à la fois"
)]
async fn session<P: PacketPool>(server: &Server<'_>, conn: &GattConnection<'_, '_, P>) {
    let rx = server.uart.rx.handle;
    let mut parseur = Parseur::default();
    let mut controle = Controle::default();
    CLIGNOTER.signal((1, 0));
    ECRAN.signal(Vue::Connecte {
        banque: controle.banque,
    });

    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[ble] téléphone déconnecté : {reason:?}");
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                let mut identification = false;
                let reply = match event {
                    GattEvent::Write(event) if event.handle() == rx => {
                        event.with_data(|_, data| {
                            debug!("[ble] reçu {data:02x?}");
                            for &octet in data {
                                match parseur.pousser(octet) {
                                    Some(Message::Manette(m)) => controle.appliquer(m),
                                    Some(Message::DemandeIdentification) => {
                                        info!("[dabble] demande d'identification");
                                        identification = true;
                                    }
                                    None => {}
                                }
                            }
                        });
                        event.accept_unprocessed()
                    }
                    GattEvent::Write(event) => {
                        debug!("[ble] écriture sur l'attribut {}", event.handle());
                        event.accept()
                    }
                    event => event.accept(),
                };
                match reply {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[ble] réponse impossible : {e:?}"),
                }
                if identification
                    && let Err(e) = server
                        .uart
                        .tx
                        .notify_raw(conn, &dabble::BOARD_ID_ESP32, false)
                        .await
                {
                    warn!("[ble] identification non envoyée : {e:?}");
                }
            }
            _ => {}
        }
    }
}

/// État de la manette entre deux trames Dabble.
#[derive(Default)]
struct Controle {
    avant: Manette,
    banque: usize,
    /// Code répété en ce moment (marche ou action maintenue).
    pas: Option<u32>,
    /// Instants d'appui sur Select et Start : la navigation se fait au relâchement,
    /// vers l'avant sur un appui court, vers l'arrière sur un appui long.
    appui_select: Option<Instant>,
    appui_start: Option<Instant>,
}

impl Controle {
    /// Traduit l'état de la manette en commandes i-Sobot.
    fn appliquer(&mut self, m: Manette) {
        if m == self.avant {
            return;
        }
        info!("[dabble] {m:?}");
        let appuis = m.boutons & !self.avant.boutons;
        let relaches = self.avant.boutons & !m.boutons;
        self.avant = m;
        self.naviguer(appuis, relaches);

        let mut maintenue = None;
        let boutons = [bouton::TRIANGLE, bouton::ROND, bouton::CROIX, bouton::CARRE];
        for (i, (bit, action)) in boutons
            .into_iter()
            .zip(BANQUES[self.banque].boutons)
            .enumerate()
        {
            let Some(action) = action else { continue };
            if appuis & bit != 0 {
                ECRAN.signal(Vue::Banque {
                    banque: self.banque,
                    actif: Some(i),
                });
            }
            if action.maintenir {
                if m.boutons & bit != 0 {
                    maintenue = Some(action.code);
                }
            } else if appuis & bit != 0 {
                envoyer(action.code);
            }
        }

        let pas = marche(m.direction).or(maintenue);
        if pas != self.pas {
            self.pas = pas;
            MARCHE.signal(pas);
        }
    }

    /// Select et Start agissent au relâchement : appui court = suivant, appui long = précédent.
    fn naviguer(&mut self, appuis: u8, relaches: u8) {
        if appuis & bouton::START != 0 {
            self.appui_start = Some(Instant::now());
        }
        if relaches & bouton::START != 0
            && let Some(debut) = self.appui_start.take()
        {
            self.changer_banque(if debut.elapsed() >= APPUI_LONG {
                categorie_precedente(self.banque)
            } else {
                categorie_suivante(self.banque)
            });
        }
        if appuis & bouton::SELECT != 0 {
            self.appui_select = Some(Instant::now());
        }
        if relaches & bouton::SELECT != 0
            && let Some(debut) = self.appui_select.take()
        {
            self.changer_banque(if debut.elapsed() >= APPUI_LONG {
                banque_precedente(self.banque)
            } else {
                banque_suivante(self.banque)
            });
        }
    }

    fn changer_banque(&mut self, banque: usize) {
        // Sans écran : les clignotements longs signalent un changement de catégorie.
        let categorie = numero_categorie(banque);
        let longs = if categorie != numero_categorie(self.banque) {
            categorie
        } else {
            0
        };
        CLIGNOTER.signal((longs, position(banque)));

        self.banque = banque;
        ECRAN.signal(Vue::Banque {
            banque,
            actif: None,
        });
        info!(
            "Catégorie {} ({}), banque {}",
            numero_categorie(banque),
            BANQUES[banque].categorie,
            position(banque)
        );
    }
}

fn envoyer(code: u32) {
    if ACTIONS.try_send(code).is_err() {
        warn!("Trop d'actions en attente, {code} ignorée");
    }
}

/// Code de marche correspondant à la direction, `None` pour rester sur place.
fn marche(direction: Direction) -> Option<u32> {
    match direction {
        Direction::Croix(f) => match (
            f & fleche::HAUT != 0,
            f & fleche::BAS != 0,
            f & fleche::GAUCHE != 0,
            f & fleche::DROITE != 0,
        ) {
            (true, false, false, false) => Some(cmd::FORWARD),
            (true, false, true, false) => Some(cmd::FLEFT),
            (true, false, false, true) => Some(cmd::FRIGHT),
            (false, true, false, false) => Some(cmd::BACKWARD),
            (false, true, true, false) => Some(cmd::BLEFT),
            (false, true, false, true) => Some(cmd::BRIGHT),
            (false, false, true, false) => Some(cmd::FCOUNTER),
            (false, false, false, true) => Some(cmd::FCLOCKWISE),
            _ => None,
        },
        // Zone morte au centre du joystick, puis 8 secteurs de 45° en partant de la droite.
        Direction::Joystick { angle, rayon } if rayon >= 3 => {
            const SECTEURS: [u32; 8] = [
                cmd::SIDERIGHT,
                cmd::FRIGHT,
                cmd::FORWARD,
                cmd::FLEFT,
                cmd::SIDELEFT,
                cmd::BLEFT,
                cmd::BACKWARD,
                cmd::BRIGHT,
            ];
            Some(SECTEURS[usize::from((angle + 22) % 360 / 45)])
        }
        _ => None,
    }
}

/// Émet les actions dès qu'elles arrivent, et répète le code de marche tant qu'il est actif.
#[embassy_executor::task]
async fn emetteur_ir(mut emetteur: rmt::Channel<'static, Async, Tx>) {
    let mut pas: Option<u32> = None;
    let mut prochaine = Instant::now();
    loop {
        let code = match pas {
            Some(code_marche) => {
                match select3(ACTIONS.receive(), MARCHE.wait(), Timer::at(prochaine)).await {
                    Either3::First(action) => action,
                    Either3::Second(nouveau) => {
                        pas = nouveau;
                        continue;
                    }
                    Either3::Third(()) => code_marche,
                }
            }
            None => match select(ACTIONS.receive(), MARCHE.wait()).await {
                Either::First(action) => action,
                Either::Second(nouveau) => {
                    pas = nouveau;
                    continue;
                }
            },
        };
        Timer::at(prochaine).await;
        debug!("[ir] émission {code:#08x}");
        if let Err(e) = emetteur
            .transmit(&ir::trame(ir::sur_canal(code, CANAL)))
            .await
        {
            warn!("[ir] échec d'émission : {e:?}");
        }
        prochaine = Instant::now() + ECART_TRAMES;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "le futur de la tâche (tampon d'image compris) vit dans le stockage statique d'embassy"
)]
mod affichage {
    use super::*;

    #[embassy_executor::task]
    pub async fn ecran_task(i2c: I2c<'static, Async>) {
        let mut ecran = Ssd1306Async::new(
            I2CDisplayInterface::new(i2c),
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();
        // L'écran peut être branché après le démarrage : on réessaie jusqu'à ce qu'il réponde.
        let mut signale = false;
        while let Err(e) = ecran.init().await {
            if !signale {
                warn!("[écran] pas de réponse sur l'I2C ({e:?}), nouvel essai toutes les 2 s");
                signale = true;
            }
            Timer::after_secs(2).await;
        }
        info!("[écran] prêt");

        let mut vue = Vue::Demarrage;
        loop {
            ecran.clear_buffer();
            let _ = ecran::dessiner(&mut ecran, vue);
            if let Err(e) = ecran.flush().await {
                warn!("[écran] erreur d'affichage : {e:?}");
            }
            // Vues passagères : « Connecté ! » pendant 1 s, surbrillance d'une action pendant 400 ms.
            vue = match vue {
                Vue::Connecte { banque } => {
                    puis(
                        Duration::from_secs(1),
                        Vue::Banque {
                            banque,
                            actif: None,
                        },
                    )
                    .await
                }
                Vue::Banque {
                    banque,
                    actif: Some(_),
                } => {
                    puis(
                        Duration::from_millis(400),
                        Vue::Banque {
                            banque,
                            actif: None,
                        },
                    )
                    .await
                }
                _ => ECRAN.wait().await,
            };
        }
    }

    /// Prochaine vue demandée, ou `ensuite` si rien n'arrive pendant `delai`.
    async fn puis(delai: Duration, ensuite: Vue) -> Vue {
        match select(ECRAN.wait(), Timer::after(delai)).await {
            Either::First(nouvelle) => nouvelle,
            Either::Second(()) => ensuite,
        }
    }
}
use affichage::ecran_task;

#[embassy_executor::task]
async fn led_task(mut led: Output<'static>) {
    loop {
        let (longs, courts) = CLIGNOTER.wait().await;
        for (fois, duree_ms) in [(longs, 600), (courts, 150)] {
            for _ in 0..fois {
                led.set_high();
                Timer::after_millis(duree_ms).await;
                led.set_low();
                Timer::after_millis(250).await;
            }
        }
    }
}
