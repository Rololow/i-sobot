//! Banques d'actions de la manette, générées à la compilation depuis `manette/banques.csv`
//! (actions maintenues : colonne `maintenir` de `manette/commandes.csv`).

use crate::isobot::cmd;

#[derive(Clone, Copy)]
pub struct Action {
    pub code: u32,
    /// Répétée tant que le bouton est appuyé (pas chassés, tête…), sinon envoyée une fois.
    pub maintenir: bool,
}

/// Actions des boutons △ ○ ✕ □.
pub struct Banque {
    pub categorie: &'static str,
    pub boutons: [Option<Action>; 4],
}

include!(concat!(env!("OUT_DIR"), "/banques.rs"));

fn meme_categorie(a: usize, b: usize) -> bool {
    BANQUES[a].categorie == BANQUES[b].categorie
}

fn debut_categorie(banque: usize) -> usize {
    let mut i = banque;
    while i > 0 && meme_categorie(i - 1, banque) {
        i -= 1;
    }
    i
}

/// Première banque de la catégorie suivante (revient à la première catégorie après la dernière).
pub fn categorie_suivante(banque: usize) -> usize {
    let mut i = banque + 1;
    while i < BANQUES.len() && meme_categorie(i, banque) {
        i += 1;
    }
    if i < BANQUES.len() { i } else { 0 }
}

/// Banque suivante dans la même catégorie (revient à la première après la dernière).
pub fn banque_suivante(banque: usize) -> usize {
    let suivante = banque + 1;
    if suivante < BANQUES.len() && meme_categorie(suivante, banque) {
        suivante
    } else {
        debut_categorie(banque)
    }
}

/// Numéro de la catégorie, à partir de 1.
pub fn numero_categorie(banque: usize) -> u8 {
    1 + (1..=banque).filter(|&i| !meme_categorie(i - 1, i)).count() as u8
}

/// Position de la banque dans sa catégorie, à partir de 1.
pub fn position(banque: usize) -> u8 {
    (banque - debut_categorie(banque) + 1) as u8
}
