//! Protocole infrarouge du robot i-Sobot (Tomy).
//!
//! Décodé par Miles Moody (bibliothèque Arduino « Isobot », 2008) et recoupé
//! avec d'autres implémentations indépendantes (Jufino, esayers, imchipwood) :
//!
//! - porteuse ~38 kHz ;
//! - en-tête : 2550 µs de porteuse ;
//! - 22 bits, bit de poids fort en premier. Chaque bit = un silence
//!   (550 µs → 0, 1050 µs → 1) suivi de 550 µs de porteuse ;
//! - mot de 22 bits : `[canal:1][0][1][checksum:3][commande:8][paramètre:8]`.

use esp_hal::{gpio::Level, rmt::PulseCode};

pub const HEADER_US: u16 = 2550;
pub const MARK_US: u16 = 550;
pub const ZERO_US: u16 = 550;
pub const ONE_US: u16 = 1050;

const FRAME_BITS: usize = 22;
/// 22 bits + dernière porteuse + marqueur de fin.
pub const FRAME_LEN: usize = FRAME_BITS + 2;

/// Canal choisi sur la télécommande (interrupteur A/B) : le robot n'obéit qu'au sien.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Canal {
    A,
    B,
}

/// Checksum sur 3 bits : somme des trois octets du mot, puis somme de ses groupes de 3 bits.
const fn checksum(word: u32) -> u32 {
    let mut sum = ((word >> 16) & 0xFF) + ((word >> 8) & 0xFF) + (word & 0xFF);
    let mut c = 0;
    let mut i = 0;
    while i < 3 {
        c += sum & 7;
        sum >>= 3;
        i += 1;
    }
    c & 7
}

/// Les codes de [`cmd`] sont ceux du canal A ; recalcule le mot pour le canal voulu.
pub const fn sur_canal(code: u32, canal: Canal) -> u32 {
    let base = code & !(1 << 21) & !(7 << 16);
    let base = match canal {
        Canal::A => base,
        Canal::B => base | (1 << 21),
    };
    base | (checksum(base) << 16)
}

/// Impulsions RMT d'une trame, en µs (canal RMT réglé à 1 tick = 1 µs, porteuse sur le niveau haut).
pub fn trame(code: u32) -> [PulseCode; FRAME_LEN] {
    let mut pulses = [PulseCode::end_marker(); FRAME_LEN];
    let mut mark = HEADER_US;
    for (i, pulse) in pulses.iter_mut().take(FRAME_BITS).enumerate() {
        let bit = (code >> (FRAME_BITS - 1 - i)) & 1;
        let space = if bit == 1 { ONE_US } else { ZERO_US };
        *pulse = PulseCode::new(Level::High, mark, Level::Low, space);
        mark = MARK_US;
    }
    pulses[FRAME_BITS] = PulseCode::new(Level::High, MARK_US, Level::Low, 0);
    pulses
}

/// Mot de 22 bits (canal A) d'une commande à un seul octet, avec le paramètre 0x03
/// utilisé par imchipwood (les codes de Moody utilisent 0x00, le robot accepte les deux).
pub const fn commande(octet: u8) -> u32 {
    sur_canal((1 << 19) | (octet as u32) << 8 | 0x03, Canal::A)
}

/// Codes des commandes (canal A), noms d'origine de Miles Moody.
pub mod cmd {
    pub const FORWARD: u32 = 898819;
    pub const BACKWARD: u32 = 964611;
    pub const SIDERIGHT: u32 = 703491;
    pub const SIDELEFT: u32 = 637699;
    pub const FLEFT: u32 = 1030403;
    pub const FRIGHT: u32 = 571907;
    pub const BLEFT: u32 = 834819;
    pub const BRIGHT: u32 = 900611;
    pub const FCLOCKWISE: u32 = 966403;
    pub const FCOUNTER: u32 = 1032195;
    pub const BCLOCKWISE: u32 = 573699;
    pub const BCOUNTER: u32 = 639491;
    pub const HEADLEFT: u32 = 907015;
    pub const HEADRIGHT: u32 = 775948;
    pub const LEANFORWARD: u32 = 841478;
    pub const LEANBACK: u32 = 1038081;
    pub const LPUNCH: u32 = 922368;
    pub const R12: u32 = 661248;
    pub const LCHOP: u32 = 858368;
    pub const SIDECHOPL: u32 = 663040;
    pub const COMBOPUNCH: u32 = 597248;
    pub const RPUNCH: u32 = 988160;
    pub const RCHOP: u32 = 924160;
    pub const L12: u32 = 792576;
    pub const SIDECHOPR: u32 = 728832;
    pub const LBACKHAND: u32 = 529664;
    pub const DOUBLECHOP: u32 = 989952;
    pub const DOUBLEBACKHAND: u32 = 925952;
    pub const SLAPPING: u32 = 860160;
    pub const RBACKHAND: u32 = 595456;
    pub const UPPERCHOP: u32 = 531456;
    pub const ROUNDHOUSEL: u32 = 991744;
    pub const ROUNDHOUSER: u32 = 533248;
    pub const FORWARDKICKL: u32 = 599040;
    pub const FORWARDKICKR: u32 = 664832;
    pub const SIDEKICKL: u32 = 730624;
    pub const ROUNDHOUSELR: u32 = 666624;
    pub const FORWARDKICKLR: u32 = 732416;
    pub const COMBOKICK: u32 = 798208;
    pub const SIDEKICKR: u32 = 796416;
    pub const BACKKICKL: u32 = 927744;
    pub const BACKKICKR: u32 = 993536;
    pub const HIGHKICKL: u32 = 864000;
    pub const HIGHKICKR: u32 = 995328;
    pub const SPLITS1: u32 = 536832;
    pub const GUARDL: u32 = 602624;
    pub const GUARDR: u32 = 668416;
    pub const DOUBLEGUARD1: u32 = 734208;
    pub const DOUBLEGUARD2: u32 = 800000;
    pub const DODGEL: u32 = 865792;
    pub const DODGER: u32 = 931584;
    pub const DUCK: u32 = 604160;
    pub const SWAYBACK: u32 = 669952;
    pub const UPBLOCK: u32 = 735744;
    pub const SPLITS2: u32 = 801536;
    pub const COMBOBLOCK: u32 = 867328;
    pub const ZERO: u32 = 1034752;
    pub const HOMEPOSITION: u32 = 775424;
    pub const SOUNDOFF: u32 = 840451;
    pub const AFFIRM: u32 = 540416;
    pub const DISAGREE: u32 = 803328;
    pub const GOODMORNING: u32 = 934912;
    pub const GREET1: u32 = 1000704;
    pub const GREET2: u32 = 608000;
    pub const GREET3: u32 = 739328;
    pub const GREET4: u32 = 805120;
    pub const BYE1: u32 = 870912;
    pub const BYE2: u32 = 936704;
    pub const BYE3: u32 = 1002496;
    pub const BYE4: u32 = 544000;
    pub const BYE5: u32 = 542208;
    pub const RESPECT: u32 = 869120;
    pub const THANKS1: u32 = 609792;
    pub const THANKS2: u32 = 675584;
    pub const LOVE1: u32 = 872704;
    pub const LOVE2: u32 = 938496;
    pub const LOVE3: u32 = 1004288;
    pub const STANDUPFRONT: u32 = 933120;
    pub const STANDUPBACK: u32 = 998912;
    pub const EXCITED1: u32 = 743168;
    pub const EXCITED2: u32 = 874496;
    pub const EXCITED3: u32 = 940288;
    pub const EXCITED4: u32 = 618752;
    pub const PARTY: u32 = 677376;
    pub const AMAZED: u32 = 750336;
    pub const REGRET1: u32 = 547584;
    pub const REGRET2: u32 = 744960;
    pub const REGRET3: u32 = 810752;
    pub const WORRY: u32 = 679168;
    pub const PAIN1: u32 = 1007872;
    pub const PAIN2: u32 = 615168;
    pub const BEG1: u32 = 942080;
    pub const BEG2: u32 = 880128;
    pub const MERRY: u32 = 552960;
    pub const HILARIOUS: u32 = 1013504;
    pub const HIDENSEEK: u32 = 613376;
    pub const YOULIKE: u32 = 682752;
    pub const MYSTERY5: u32 = 748544;
    pub const TIPSY: u32 = 814336;
    pub const TICKLEME: u32 = 686080;
    pub const TIREDFEET: u32 = 751872;
    pub const NEEDABREAK: u32 = 817664;
    pub const WAVE1: u32 = 883456;
    pub const WAVE2: u32 = 949248;
    pub const APPLAUSE: u32 = 947712;
    pub const MYSTERY6: u32 = 945920;
    pub const TOOSEXY: u32 = 1015040;
    pub const CLINK: u32 = 556544;
    pub const RELAX: u32 = 753664;
    pub const SOCCER1: u32 = 885248;
    pub const SOCCER2: u32 = 600832;
    pub const SOCCER3: u32 = 535040;
    pub const LIFT: u32 = 819456;
    pub const COUNTONME: u32 = 951040;
    pub const ARTICULATION: u32 = 1016832;
    pub const SHOWOFF1: u32 = 558336;
    pub const SHOWOFF2: u32 = 624128;
    pub const SHOWOFF3: u32 = 689920;
    pub const SHOWOFF4: u32 = 821248;
    pub const COMINTHROUGH: u32 = 887040;
    pub const CATCH: u32 = 1006080;
    pub const POSE1: u32 = 771840;
    pub const POSE2: u32 = 903168;
    pub const POSE3: u32 = 968960;
    pub const MYSTERY1: u32 = 684544;
    pub const MYSTERY2: u32 = 816128;
    pub const MYSTERY3: u32 = 881920;
    pub const MYSTERY4: u32 = 549376;
    pub const FORWARDSOMERSAULT: u32 = 952832;
    pub const HEADSTANDEXERCISES: u32 = 1018624;
    pub const EXERCISES: u32 = 560128;
    pub const AIRDRUM: u32 = 625920;
    pub const AIRGUITAR: u32 = 691712;
    pub const RANDOMPERFORMANCE1: u32 = 954624;
    pub const RANDOMANIMAL: u32 = 627712;
    pub const TROPICALDANCE: u32 = 825088;
    pub const GIANTROBOT: u32 = 956416;
    pub const WESTERN: u32 = 1022208;
    pub const RANDOMPERFORMANCE2: u32 = 629504;

    // Commandes absentes de la table de Moody (imchipwood, d'après le rapport de M. Sunardi)
    pub const STOP: u32 = super::commande(0x00);
    pub const BANZAI3: u32 = super::commande(0x8F);
    pub const BANZAI: u32 = super::commande(0x7F);
    pub const CHEER: u32 = super::commande(0x90);
    pub const AAAA: u32 = super::commande(0x9A);
    pub const SWANLAKE: u32 = super::commande(0xB1);
    pub const DISCO: u32 = super::commande(0xB2);
    pub const MOONWALK: u32 = super::commande(0xB3);
    pub const GORILLA: u32 = super::commande(0x96);
    pub const DOG: u32 = super::commande(0x92);
    pub const CAR: u32 = super::commande(0x93);
    pub const ROOSTER: u32 = super::commande(0x95);
    pub const LOOKOUT: u32 = super::commande(0xA1);
    pub const STORY1: u32 = super::commande(0xA2);
    pub const STORY2: u32 = super::commande(0xA3);
    pub const GOODMORNING2: u32 = super::commande(0xA4);
    pub const LETSPLAY: u32 = super::commande(0xA5);
    pub const OOPS: u32 = super::commande(0xA6);
    pub const GOOUT: u32 = super::commande(0xA7);
    pub const ROUND: u32 = super::commande(0xA8);
    pub const INTRODUCTION: u32 = super::commande(0xA9);
    pub const ATYOURSERVICE: u32 = super::commande(0xAA);
    pub const SMELLS: u32 = super::commande(0xAB);
    pub const CLOSE: u32 = super::commande(0xAC);
    pub const PIECEOFME: u32 = super::commande(0xAD);
    pub const RUNFORYOURLIFE: u32 = super::commande(0xAE);
    pub const TOOYOUNG: u32 = super::commande(0xAF);
    pub const ALLYOURBASE: u32 = super::commande(0x41);
    pub const BEARHUG: u32 = super::commande(0x54);
    pub const TURNON: u32 = super::commande(0x01);
    pub const ACTIVATED: u32 = super::commande(0x02);
    pub const READY: u32 = super::commande(0x03);
    pub const BEEP: u32 = super::commande(0x6A);
    pub const HUMMING: u32 = super::commande(0x0F);
    pub const COUGH: u32 = super::commande(0x10);
    pub const TIRED: u32 = super::commande(0x11);
    pub const SLEEPY: u32 = super::commande(0x12);
    pub const IDLE: u32 = super::commande(0x0B);
    pub const FART: u32 = super::commande(0x40);
    pub const THANKYOUSIR: u32 = super::commande(0xEA);
    pub const ILOVEYOU: u32 = super::commande(0xEB);
    pub const SHOOTRIGHT: u32 = super::commande(0x64);
    pub const SHOOTRIGHT2: u32 = super::commande(0x68);
    pub const SHOOT2: u32 = super::commande(0x69);
    pub const THREEBEEPS: u32 = super::commande(0xEC);
    pub const RCCONFIRM: u32 = super::commande(0x04);
    pub const RCPROMPT: u32 = super::commande(0x05);
    pub const MODEPROMPT: u32 = super::commande(0x06);
    pub const REPEAT1: u32 = super::commande(0xB4);
    pub const REPEAT2: u32 = super::commande(0xB5);
    pub const REPEAT3: u32 = super::commande(0xB6);
    pub const THREEBEEPSSLIDE: u32 = super::commande(0xEE);
    pub const WAHOO: u32 = super::commande(0x65);
    pub const HOOAH: u32 = super::commande(0x66);
    pub const KAPWING: u32 = super::commande(0x67);
    pub const HEADSMASH: u32 = super::commande(0xC5);
    pub const HEADHIT: u32 = super::commande(0xC6);
    pub const PLAYDEAD: u32 = super::commande(0xED);
    pub const BENDBACK: u32 = super::commande(0xD8);
    pub const BENDFORWARD: u32 = super::commande(0xDD);
    pub const HEADLEFT60: u32 = super::commande(0xDE);
    pub const HEADLEFT45: u32 = super::commande(0xDF);
    pub const HEADLEFT30: u32 = super::commande(0xE0);
    pub const HEADRIGHT30: u32 = super::commande(0xE1);
    pub const HEADRIGHT45: u32 = super::commande(0xE2);
    pub const HEADRIGHT60: u32 = super::commande(0xE3);
    pub const HEADSCAN: u32 = super::commande(0xE7);
    pub const ARMTEST: u32 = super::commande(0xE8);
    pub const LEGTEST: u32 = super::commande(0xE9);
    pub const ACT43A: u32 = super::commande(0x50);
    pub const ACT12B: u32 = super::commande(0x55);
    pub const MODERC: u32 = super::commande(0x07);
    pub const MODEPROG: u32 = super::commande(0x08);
    pub const MODESON: u32 = super::commande(0x09);
    pub const MODEVOCAL: u32 = super::commande(0x0A);
}
