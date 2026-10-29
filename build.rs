fn main() {
    generer_banques();
    linker_be_nice();
    check_xtensa_linker_available();
    // Symboles ROM Bluetooth manquants dans esp-rom-sys (voir ld/bt_rom_fix.x)
    let ld_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ld");
    println!("cargo:rustc-link-search={}", ld_dir.display());
    println!("cargo:rerun-if-changed=ld/bt_rom_fix.x");
    println!("cargo:rustc-link-arg=-Tbt_rom_fix.x");
    // make sure linkall.x is the last linker script (otherwise might cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

/// Génère `BANQUES` (voir src/manette.rs) depuis les CSV de `manette/`, partagés avec la fiche Typst.
fn generer_banques() {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("manette");
    println!("cargo:rerun-if-changed=manette/banques.csv");
    println!("cargo:rerun-if-changed=manette/commandes.csv");
    let lire = |nom: &str| {
        std::fs::read_to_string(dossier.join(nom))
            .unwrap_or_else(|e| panic!("lecture de manette/{nom} : {e}"))
    };
    let lignes = |texte: &str| -> Vec<Vec<String>> {
        texte
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split(',').map(|c| c.trim().to_string()).collect())
            .collect()
    };

    // commande, libellé, maintenir, libellé court pour l'écran
    let commandes = lignes(&lire("commandes.csv"));
    for c in &commandes {
        assert_eq!(
            c.len(),
            4,
            "manette/commandes.csv : 4 colonnes attendues, lu {c:?}"
        );
    }

    let mut rs = String::from("pub const BANQUES: &[Banque] = &[\n");
    for banque in lignes(&lire("banques.csv")) {
        assert_eq!(
            banque.len(),
            5,
            "manette/banques.csv : 5 colonnes attendues, lu {banque:?}"
        );
        let boutons: Vec<String> = banque[1..]
            .iter()
            .map(|nom| {
                if nom.is_empty() {
                    return "None".to_string();
                }
                let c = commandes
                    .iter()
                    .find(|c| c[0] == *nom)
                    .unwrap_or_else(|| panic!("« {nom} » absent de manette/commandes.csv"));
                format!(
                    "Some(Action {{ code: cmd::{}, maintenir: {}, ecran: {:?} }})",
                    nom.to_uppercase(),
                    c[2] == "oui",
                    c[3]
                )
            })
            .collect();
        rs += &format!(
            "    Banque {{ categorie: {:?}, boutons: [{}] }},\n",
            banque[0],
            boutons.join(", ")
        );
    }
    rs += "];\n";
    let sortie = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("banques.rs");
    std::fs::write(sortie, rs).unwrap();
}

#[cfg(unix)]
fn check_xtensa_linker_available() {
    println!("cargo:rerun-if-env-changed=PATH");

    let target = std::env::var("TARGET").unwrap_or_default();
    println!(
        "cargo:rerun-if-env-changed={}",
        cargo_linker_env_var(&target)
    );

    let linker = xtensa_linker(&target);

    let error = match std::process::Command::new(&linker)
        .arg("--version")
        .output()
    {
        Ok(_) => return,
        Err(error) => error,
    };

    let export_file = std::env::var("HOME")
        .map(|home| format!("{home}/export-esp.sh"))
        .unwrap_or_else(|_| "$HOME/export-esp.sh".to_string());

    panic!(
        "Xtensa linker `{linker}` was not found in PATH or could not be executed: {error}.\n\n\
         Xtensa targets on Unix require espup's environment export file to be sourced before building.\n\
         Try: `source {export_file}`\n\n\
         If the export file does not exist, run `espup install` first.\n\
         For more details, see:\n\
         https://github.com/esp-rs/espup?tab=readme-ov-file#environment-variables-setup"
    );
}

#[cfg(not(unix))]
fn check_xtensa_linker_available() {}

#[cfg(unix)]
fn xtensa_linker(target: &str) -> String {
    if let Some(linker) = cargo_configured_linker(target) {
        return linker;
    }

    target
        .strip_prefix("xtensa-")
        .and_then(|target| target.strip_suffix("-none-elf"))
        .map(|chip| format!("xtensa-{chip}-elf-gcc"))
        .unwrap_or_else(|| "xtensa-esp32-elf-gcc".to_string())
}

#[cfg(unix)]
fn cargo_configured_linker(target: &str) -> Option<String> {
    std::env::var(cargo_linker_env_var(target)).ok()
}

#[cfg(unix)]
fn cargo_linker_env_var(target: &str) -> String {
    let target = target.replace('-', "_").to_ascii_uppercase();
    format!("CARGO_TARGET_{target}_LINKER")
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                what if what.starts_with("_defmt_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!("💡 Is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                what if what.starts_with("esp_rtos_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                "free"
                | "malloc"
                | "calloc"
                | "get_free_internal_heap_size"
                | "malloc_internal"
                | "realloc_internal"
                | "calloc_internal"
                | "free_internal" => {
                    eprintln!();
                    eprintln!(
                        "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
                    );
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}
