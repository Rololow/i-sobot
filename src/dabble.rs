//! Protocole de l'application Dabble (STEMpedia), transporté sur le service BLE « Nordic UART ».
//!
//! Trame : `FF | module | fonction | nb_args | (taille, données…)… | 00`.
//! Repris de la bibliothèque officielle DabbleESP32.

const START: u8 = 0xFF;
const END: u8 = 0x00;

const MODULE_DABBLE: u8 = 0x00;
const MODULE_GAMEPAD: u8 = 0x01;

const BOARD_ID_REQUEST: u8 = 0x03;
const GAMEPAD_DIGITAL: u8 = 0x01;
const GAMEPAD_ANALOG: u8 = 0x02;
const GAMEPAD_ACCL: u8 = 0x03;

/// Réponse à la demande d'identification de l'app : carte « ESP32 », bibliothèque 1.5.1.
pub const BOARD_ID_ESP32: [u8; 10] = [START, MODULE_DABBLE, BOARD_ID_REQUEST, 1, 4, 0x06, 1, 5, 1, END];

/// Bits de l'octet « boutons » de la manette.
pub mod bouton {
    pub const START: u8 = 1 << 0;
    pub const SELECT: u8 = 1 << 1;
    pub const TRIANGLE: u8 = 1 << 2;
    pub const ROND: u8 = 1 << 3;
    pub const CROIX: u8 = 1 << 4;
    pub const CARRE: u8 = 1 << 5;
}

/// Bits de la croix directionnelle (mode « Digital »).
pub mod fleche {
    pub const HAUT: u8 = 1 << 0;
    pub const BAS: u8 = 1 << 1;
    pub const GAUCHE: u8 = 1 << 2;
    pub const DROITE: u8 = 1 << 3;
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Direction {
    #[default]
    Aucune,
    /// Croix directionnelle : combinaison de bits de [`fleche`].
    Croix(u8),
    /// Joystick (modes « Joystick » et « Accéléromètre ») :
    /// angle en degrés (0 = droite, 90 = haut), rayon de 0 à 7.
    Joystick { angle: u16, rayon: u8 },
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Manette {
    /// Combinaison de bits de [`bouton`].
    pub boutons: u8,
    pub direction: Direction,
}

pub enum Message {
    Manette(Manette),
    DemandeIdentification,
}

/// Reconstitue les trames : une écriture BLE peut contenir une trame partielle, ou plusieurs.
pub struct Parseur {
    buf: [u8; 64],
    len: usize,
}

impl Default for Parseur {
    fn default() -> Self {
        Self { buf: [0; 64], len: 0 }
    }
}

impl Parseur {
    /// Ajoute un octet reçu ; renvoie un message dès qu'une trame complète et valide est arrivée.
    pub fn pousser(&mut self, octet: u8) -> Option<Message> {
        if self.len == 0 && octet != START {
            return None;
        }
        self.buf[self.len] = octet;
        self.len += 1;

        let Some(taille) = self.taille_attendue() else {
            if self.len == self.buf.len() {
                self.len = 0;
            }
            return None;
        };
        if taille > self.buf.len() {
            self.len = 0;
            return None;
        }
        if self.len < taille {
            return None;
        }
        self.len = 0;
        let trame = &self.buf[..taille];
        if trame[taille - 1] != END {
            return None;
        }
        decoder(trame)
    }

    /// Taille totale de la trame en cours, dès que les octets reçus permettent de la connaître.
    fn taille_attendue(&self) -> Option<usize> {
        let recu = &self.buf[..self.len];
        let nb_args = *recu.get(3)?;
        let mut i = 4;
        for _ in 0..nb_args {
            i += 1 + *recu.get(i)? as usize;
        }
        Some(i + 1)
    }
}

fn decoder(trame: &[u8]) -> Option<Message> {
    let (module, fonction, nb_args) = (trame[1], trame[2], trame[3]);
    match module {
        MODULE_DABBLE if fonction == BOARD_ID_REQUEST => Some(Message::DemandeIdentification),
        MODULE_GAMEPAD if nb_args >= 1 && trame[4] >= 2 => {
            let (boutons, valeur) = (trame[5], trame[6]);
            let direction = match fonction {
                GAMEPAD_DIGITAL => Direction::Croix(valeur & 0x0F),
                GAMEPAD_ANALOG | GAMEPAD_ACCL => Direction::Joystick {
                    angle: u16::from(valeur >> 3) * 15,
                    rayon: valeur & 0x07,
                },
                _ => return None,
            };
            Some(Message::Manette(Manette { boutons, direction }))
        }
        _ => None,
    }
}
