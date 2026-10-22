use mnm_stats_model::History;
use std::{env, fs, path::PathBuf};

#[path = "build_tokens.rs"]
mod tokens;

fn main() {
    println!("cargo:rerun-if-changed=build_tokens.rs");
    println!("cargo:rerun-if-changed=build_tailwind.rs");
    println!("cargo:rerun-if-changed=/opt/tailwindcss/theme.css");
    println!("cargo:rerun-if-changed=style.css");
    println!("cargo:rerun-if-env-changed=MNM_STATS_TOKENS");
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let token_input = env::var_os("MNM_STATS_TOKENS")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("design-tokens.tokens.json"));
    println!("cargo:rerun-if-changed={}", token_input.display());
    let token_source = fs::read_to_string(&token_input)
        .unwrap_or_else(|e| panic!("cannot read tokens {}: {e}", token_input.display()));
    let styles = fs::read_to_string(manifest.join("style.css")).unwrap();
    let generated = tokens::generate(&token_source, &styles)
        .unwrap_or_else(|e| panic!("invalid tokens {}: {e}", token_input.display()));
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("tokens.rs"), generated.rust).unwrap();
    fs::write(out.join("style.css"), generated.css).unwrap();
    fs::write(out.join("tokens.resolved.json"), generated.resolved).unwrap();

    println!("cargo:rerun-if-env-changed=MNM_STATS_DEMO");
    println!(
        "cargo:rustc-env=MNM_STATS_DEMO={}",
        env::var("MNM_STATS_DEMO").unwrap_or_default()
    );
    println!("cargo:rerun-if-env-changed=MNM_STATS_HISTORY");
    let input = env::var_os("MNM_STATS_HISTORY").map_or_else(
        || {
            let local = manifest.join("../../data/history.jsonl");
            if local.is_file() {
                local
            } else {
                manifest.join("tests/fixtures/empty-history.jsonl")
            }
        },
        PathBuf::from,
    );
    println!("cargo:rerun-if-changed={}", input.display());
    let source = fs::read_to_string(&input)
        .unwrap_or_else(|e| panic!("cannot read history {}: {e}", input.display()));
    History::from_jsonl(&source)
        .unwrap_or_else(|e| panic!("invalid history {}: {e}", input.display()));
    // Keep validated input as data so growing history does not generate a large
    // constructor function for the Rust optimizer.
    fs::write(out.join("history.jsonl"), source).unwrap();
}
