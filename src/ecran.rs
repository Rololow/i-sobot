//! Contenu de l'écran OLED 128×64 : attente du téléphone, ou banque en cours.
//!
//! Écran bicolore : les 16 premières lignes sont jaunes (en-tête), les 48 suivantes bleues
//! (les 4 actions, 12 pixels chacune).

use alloc::format;
use embedded_graphics::{
    mono_font::{
        MonoTextStyle,
        iso_8859_1::{FONT_6X10, FONT_7X13_BOLD, FONT_9X15_BOLD},
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::manette::{BANQUES, numero_categorie, position};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vue {
    Attente,
    Banque {
        banque: usize,
        /// Bouton (0 à 3 : △ ○ ✕ □) dont l'action vient d'être envoyée, affiché en surbrillance.
        actif: Option<usize>,
    },
}

const LARGEUR: i32 = 128;
/// Hauteur de la zone jaune.
const HAUT_ENTETE: i32 = 16;
const HAUT_LIGNE: i32 = 12;

pub fn dessiner<D: DrawTarget<Color = BinaryColor>>(d: &mut D, vue: Vue) -> Result<(), D::Error> {
    match vue {
        Vue::Attente => attente(d),
        Vue::Banque { banque, actif } => banque_en_cours(d, banque, actif),
    }
}

fn attente<D: DrawTarget<Color = BinaryColor>>(d: &mut D) -> Result<(), D::Error> {
    let centre = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();
    let titre = MonoTextStyle::new(&FONT_9X15_BOLD, BinaryColor::On);
    let petit = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    Text::with_text_style("i-Sobot", Point::new(LARGEUR / 2, 1), titre, centre).draw(d)?;
    Text::with_text_style("En attente du", Point::new(LARGEUR / 2, 28), petit, centre).draw(d)?;
    Text::with_text_style("téléphone...", Point::new(LARGEUR / 2, 42), petit, centre).draw(d)?;
    Ok(())
}

fn banque_en_cours<D: DrawTarget<Color = BinaryColor>>(
    d: &mut D,
    banque: usize,
    actif: Option<usize>,
) -> Result<(), D::Error> {
    let b = &BANQUES[banque];
    let nb_banques = BANQUES
        .iter()
        .filter(|x| x.categorie == b.categorie)
        .count();

    // En-tête en négatif dans la zone jaune : « 3 Poings      2/4 »
    Rectangle::new(Point::zero(), Size::new(LARGEUR as u32, HAUT_ENTETE as u32))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(d)?;
    let negatif = MonoTextStyle::new(&FONT_7X13_BOLD, BinaryColor::Off);
    let gauche = format!("{} {}", numero_categorie(banque), b.categorie);
    Text::with_baseline(&gauche, Point::new(2, 2), negatif, Baseline::Top).draw(d)?;
    let droite = TextStyleBuilder::new()
        .alignment(Alignment::Right)
        .baseline(Baseline::Top)
        .build();
    let compte = format!("{}/{}", position(banque), nb_banques);
    Text::with_text_style(&compte, Point::new(LARGEUR - 2, 2), negatif, droite).draw(d)?;

    for (i, action) in b.boutons.iter().enumerate() {
        let y = HAUT_ENTETE + i as i32 * HAUT_LIGNE;
        let couleur = if actif == Some(i) {
            Rectangle::new(
                Point::new(0, y),
                Size::new(LARGEUR as u32, HAUT_LIGNE as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(d)?;
            BinaryColor::Off
        } else {
            BinaryColor::On
        };
        symbole(d, i, Point::new(2, y + 2), couleur)?;
        let texte = action.map_or("-", |a| a.ecran);
        Text::with_baseline(
            texte,
            Point::new(14, y + 1),
            MonoTextStyle::new(&FONT_6X10, couleur),
            Baseline::Top,
        )
        .draw(d)?;
    }
    Ok(())
}

/// △ ○ ✕ □ dessinés dans un carré de 9 pixels.
fn symbole<D: DrawTarget<Color = BinaryColor>>(
    d: &mut D,
    bouton: usize,
    coin: Point,
    couleur: BinaryColor,
) -> Result<(), D::Error> {
    let trait_ = PrimitiveStyle::with_stroke(couleur, 1);
    let (x, y) = (coin.x, coin.y);
    match bouton {
        0 => Triangle::new(
            Point::new(x + 4, y),
            Point::new(x + 8, y + 8),
            Point::new(x, y + 8),
        )
        .into_styled(trait_)
        .draw(d),
        1 => Circle::new(coin, 9).into_styled(trait_).draw(d),
        2 => {
            Line::new(coin, Point::new(x + 8, y + 8))
                .into_styled(trait_)
                .draw(d)?;
            Line::new(Point::new(x + 8, y), Point::new(x, y + 8))
                .into_styled(trait_)
                .draw(d)
        }
        _ => Rectangle::new(coin, Size::new(9, 9))
            .into_styled(trait_)
            .draw(d),
    }
}
