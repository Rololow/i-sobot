// Fiche de la manette i-Sobot (app Dabble, module GamePad).
// Les banques et les libellés viennent de banques.csv et commandes.csv,
// les mêmes fichiers que ceux compilés dans le firmware.

#let commandes = csv("commandes.csv").slice(1)
#let libelle = commandes.map(c => (c.at(0), c.at(1))).to-dict()
#let maintenues = commandes.filter(c => c.at(2) == "oui").map(c => c.at(0))
#let banques = csv("banques.csv").slice(1)

#let categories = {
  let res = ()
  for (i, b) in banques.enumerate() {
    if res.len() == 0 or res.at(-1).nom != b.at(0) {
      res.push((nom: b.at(0), debut: i + 1, fin: i + 1))
    } else {
      res.at(-1).fin = i + 1
    }
  }
  res
}

#let encre = rgb("#1d2433")
#let gris = rgb("#6b7280")
#let filet = rgb("#d9dde3")
#let fond = rgb("#f3f5f8")
#let bleu-led = rgb("#2f80ed")
#let couleurs = (
  triangle: rgb("#1f9d55"),
  rond: rgb("#d64541"),
  croix: rgb("#2f6fd6"),
  carre: rgb("#c2459e"),
)

#set document(title: "i-Sobot · manette Dabble")
#set page(
  paper: "a4",
  margin: (x: 14mm, top: 14mm, bottom: 16mm),
  footer: context [
    #set text(7.5pt, fill: gris)
    i-Sobot · manette Dabble
    #h(1fr)
    #counter(page).display("1 / 1", both: true)
  ],
)
#set text(font: ("Segoe UI", "Libertinus Serif"), size: 9.5pt, fill: encre, lang: "fr")
#set par(leading: 0.5em)

// Symboles des boutons, dessinés pour ne dépendre d'aucune police.
#let symbole(forme, taille: 8pt) = {
  let trait = 1.3pt + couleurs.at(forme)
  box(baseline: 12%, width: taille, height: taille, {
    if forme == "triangle" {
      place(polygon(stroke: trait, (taille / 2, 0pt), (taille, taille), (0pt, taille)))
    } else if forme == "rond" {
      place(circle(radius: taille / 2, stroke: trait))
    } else if forme == "croix" {
      place(line(start: (0pt, 0pt), end: (taille, taille), stroke: trait))
      place(line(start: (taille, 0pt), end: (0pt, taille), stroke: trait))
    } else {
      place(rect(width: taille, height: taille, stroke: trait))
    }
  })
}

// Clignotements de la LED : longs pour la catégorie, courts pour la banque.
// Pastilles en ligne : elles passent à la ligne si la colonne est trop étroite.
#let clignotements(n, largeur) = (
  range(n).map(_ => box(width: largeur, height: 4pt, radius: 2pt, fill: bleu-led)).join(h(2pt))
)
#let led-longs(n, largeur: 10pt) = clignotements(n, largeur)
#let led-courts(n) = clignotements(n, 4pt)

#let badge-maintenir = box(
  fill: rgb("#fff1d6"),
  inset: (x: 3pt, y: 1.5pt),
  radius: 2pt,
  text(6.5pt, fill: rgb("#9a6200"), weight: "semibold")[maintenir],
)

#let action(nom) = if nom == "" {
  text(fill: filet)[—]
} else {
  libelle.at(nom)
  if nom in maintenues { [ #badge-maintenir] }
}

#let titre-section(corps) = block(above: 1.4em, below: 0.7em, text(11pt, weight: "bold", corps))

// ───────────────────────────── Page 1 : mode d'emploi

#block(below: 0.4em, text(22pt, weight: "bold")[i-Sobot])
#text(11pt, fill: gris)[Piloter le robot depuis le téléphone : app *Dabble*, module *GamePad*]

#titre-section[Se connecter]
#grid(
  columns: (auto, 1fr),
  column-gutter: 8pt,
  row-gutter: 6pt,
  ..(
    [Allumer l'i-Sobot (réglé sur le *canal A*) et brancher l'ESP32, émetteur infrarouge tourné vers le robot. L'écran affiche *« Démarrage »* : attendre *« Prêt ! »*.],
    [Dans Dabble, toucher l'icône de connexion puis choisir *« i-Sobot »*. L'écran affiche *« Connecté ! »* puis la banque 1. Si la première tentative échoue, réessayer.],
    [Ouvrir le module *GamePad*. Si le téléphone se déconnecte, l'écran affiche *« Déconnecté »* : se reconnecter depuis Dabble.],
  )
    .enumerate()
    .map(((i, t)) => (
      box(width: 14pt, height: 14pt, radius: 7pt, fill: encre, align(center + horizon, text(8pt, fill: white, weight: "bold", str(i + 1)))),
      t,
    ))
    .flatten(),
)

#titre-section[Les commandes]
#let fleche(signe, nom) = box(
  width: 100%,
  height: 46pt,
  fill: fond,
  radius: 4pt,
  inset: 4pt,
  align(center + horizon)[
    #text(16pt, signe) \
    #text(7.5pt, libelle.at(nom))
  ],
)
#grid(
  columns: (52%, 1fr),
  column-gutter: 14pt,
  [
    #text(weight: "semibold")[Croix directionnelle] #h(4pt) #text(8pt, fill: gris)[— marche tant qu'on appuie]
    #v(2pt)
    #grid(
      columns: (1fr, 1fr, 1fr),
      gutter: 4pt,
      fleche("↖", "fleft"), fleche("↑", "forward"), fleche("↗", "fright"),
      fleche("←", "fcounter"),
      box(width: 100%, height: 46pt, align(center + horizon, text(7.5pt, fill: gris)[diagonales :\ 2 flèches\ à la fois])),
      fleche("→", "fclockwise"),
      fleche("↙", "bleft"), fleche("↓", "backward"), fleche("↘", "bright"),
    )
  ],
  [
    #set par(leading: 0.45em)
    #let ligne(cle, texte) = grid(
      columns: (58pt, 1fr),
      column-gutter: 6pt,
      align(right, text(weight: "semibold", cle)), texte,
    )
    #stack(
      spacing: 9pt,
      ligne[#symbole("triangle") #symbole("rond") #symbole("croix") #symbole("carre")][Action de la banque en cours (tableau ci-dessous).],
      ligne[Select][Banque suivante ; après la dernière banque d'une catégorie, passe à la catégorie suivante. *Appui long* (½ s) : banque précédente.],
      ligne[Start][Saute directement à la catégorie suivante. *Appui long* (½ s) : catégorie précédente.],
      ligne[Écran][Ligne 1 : catégorie et banque, par exemple #box[*3 Poings 2/4*]. Lignes 2 à 5 : actions de #symbole("triangle") #symbole("rond") #symbole("croix") #symbole("carre").],
      ligne[LED bleue][Sans écran : un clignotement *long* par numéro de catégorie quand elle change (#led-longs(3) = Poings), puis un *court* par numéro de banque (#led-courts(2) = banque 2).],
      ligne[Joystick][Mode *Joystick* de Dabble : marche dans 8 directions, pas chassés compris.],
      ligne[#badge-maintenir][Action répétée tant que le bouton reste appuyé.],
    )
  ],
)

// ───────────────────────────── Tableau des banques

#titre-section[Les #categories.len() catégories et leurs #banques.len() banques]

// Une table par catégorie, insécable : son en-tête ne se retrouve jamais seul en bas de page.
#for (k, c) in categories.enumerate() {
  block(breakable: false, below: 4pt, table(
    columns: (20pt, 72pt, 1fr, 1fr, 1fr, 1fr),
    stroke: (x, y) => (bottom: 0.5pt + filet),
    inset: (x: 5pt, y: 4.5pt),
    align: (x, y) => if x == 0 { center + horizon } else { left + horizon },
    fill: (x, y) => if y == 0 { fond },
    table.cell(colspan: 2, align: left + horizon, inset: (x: 6pt, y: 5pt))[
      #text(9.5pt, weight: "bold")[#(k + 1) · #c.nom] \
      #led-longs(k + 1, largeur: 5.5pt)
    ],
    ..("triangle", "rond", "croix", "carre").map(f => symbole(f, taille: 9pt)),
    ..for n in range(c.debut, c.fin + 1) {
      let p = n - c.debut + 1
      (text(10.5pt, weight: "bold", str(p)), led-courts(p), ..banques.at(n - 1).slice(1).map(action))
    },
  ))
}

#v(6pt)
#text(7.5pt, fill: gris)[
  Pour réorganiser les banques : modifier `isobot/manette/banques.csv` (noms de `commandes.csv`), puis lancer `cargo run --release` dans `isobot/`. Cette fiche se recompile avec `typst compile manette.typ`.
]
