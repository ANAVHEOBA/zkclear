use compliance_attestation_adapter::service::sanctions_service::SanctionsEntry;
use std::env;
use std::fs;
use std::io::Write;

fn main() {
    if let Err(e) = run() {
        eprintln!("refresh_sanctions error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let input_path = env::var("OFAC_SDN_CSV_PATH").unwrap_or_else(|_| "data/raw/ofac_sdn.csv".to_string());
    let output_path = env::var("SANCTIONS_OUTPUT_PATH").unwrap_or_else(|_| "data/sanctions.json".to_string());

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(&input_path)
        .map_err(|e| format!("failed to open input csv: {e}"))?;

    let mut entries: Vec<SanctionsEntry> = Vec::new();
    for record in reader.records() {
        let rec = match record {
            Ok(r) => r,
            Err(_) => continue,
        };
        let name = rec.get(1).unwrap_or_default().trim();
        if name.is_empty() {
            continue;
        }
        let program = rec.get(2).unwrap_or_default().trim();
        let country = rec.get(8).unwrap_or_default().trim();
        let address = rec.get(9).unwrap_or_default().trim();

        entries.push(SanctionsEntry {
            source: "OFAC".to_string(),
            program: if program.is_empty() { "SDN".to_string() } else { program.to_string() },
            name: name.to_string(),
            jurisdiction: if country.is_empty() { None } else { Some(country.to_string()) },
            address: if address.is_empty() { None } else { Some(address.to_string()) },
        });
    }

    let payload = serde_json::to_vec_pretty(&entries)
        .map_err(|e| format!("failed to serialize sanctions entries: {e}"))?;
    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create output dir: {e}"))?;
    }
    let mut file = fs::File::create(&output_path).map_err(|e| format!("failed to create output file: {e}"))?;
    file.write_all(&payload)
        .map_err(|e| format!("failed to write output file: {e}"))?;
    Ok(())
}
