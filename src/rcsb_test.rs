use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::error::Error;

const RCSB_GRAPHQL_URL: &str = "https://data.rcsb.org/graphql";

fn rcsb_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (rcsb-integration-test)")
        .build()
}

fn rcsb_cif_url(pdb_id: &str) -> String {
    format!("https://files.rcsb.org/download/{pdb_id}.cif")
}

fn rcsb_pdb_url(pdb_id: &str) -> String {
    format!("https://files.rcsb.org/download/{pdb_id}.pdb")
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, Box<dyn Error>> {
    let bytes = fetch_bytes(client, url).await?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(text)
}

async fn fetch_bytes(client: &Client, url: &str) -> Result<bytes::Bytes, Box<dyn Error>> {
    let response = client.get(url).send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(
            format!("RCSB request failed for URL '{url}' with status {status}: {body}").into(),
        );
    }

    let bytes = response.bytes().await?;
    Ok(bytes)
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphqlEnvelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

async fn rcsb_graphql_query<T: for<'de> Deserialize<'de>>(
    client: &Client,
    query: &str,
    variables: serde_json::Value,
) -> Result<T, Box<dyn Error>> {
    let body = json!({
        "query": query,
        "variables": variables,
    });

    let response = client.post(RCSB_GRAPHQL_URL).json(&body).send().await?;
    let status = response.status();

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "RCSB request failed for URL '{}' with status {}: {}",
            RCSB_GRAPHQL_URL, status, text
        )
        .into());
    }

    let envelope: GraphqlEnvelope<T> = response.json().await?;
    if !envelope.errors.is_empty() {
        let combined = envelope
            .errors
            .iter()
            .map(|error| error.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("RCSB GraphQL returned errors: {combined}").into());
    }

    envelope.data.ok_or_else(|| {
        format!("RCSB GraphQL response contained no data payload for query: {query}").into()
    })
}

#[derive(Debug, Deserialize)]
struct RcsbGraphqlData {
    entry: Option<RcsbEntry>,
}

#[derive(Debug, Deserialize)]
struct RcsbEntry {
    rcsb_id: String,
    #[serde(default)]
    r#struct: Option<RcsbStruct>,
    #[serde(default)]
    rcsb_entry_info: Option<RcsbEntryInfo>,
}

#[derive(Debug, Deserialize)]
struct RcsbStruct {
    title: String,
}

#[derive(Debug, Deserialize)]
struct RcsbEntryInfo {
    molecular_weight: Option<f64>,
}

const RCSB_ENTRY_QUERY: &str = r#"
query ($id: String!) {
  entry(entry_id: $id) {
        rcsb_id
    struct {
      title
    }
    rcsb_entry_info {
      molecular_weight
    }
  }
}
"#;

async fn fetch_rcsb_entry_metadata(
    client: &Client,
    pdb_id: &str,
) -> Result<RcsbEntry, Box<dyn Error>> {
    let data: RcsbGraphqlData =
        rcsb_graphql_query(client, RCSB_ENTRY_QUERY, json!({ "id": pdb_id }))
            .await
            .map_err(|error| {
                format!("Failed to fetch RCSB metadata for '{}': {}", pdb_id, error)
            })?;

    data.entry.ok_or_else(|| {
        format!(
            "RCSB GraphQL returned null entry payload for entry id '{}'",
            pdb_id
        )
        .into()
    })
}

#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_rcsb_mmCIF_basic_structure() -> Result<(), Box<dyn Error>> {
    let client = rcsb_client()?;
    let pdb_id = "6VXX";
    let url = rcsb_cif_url(pdb_id);
    let cif_bytes = fetch_bytes(&client, &url).await?;
    let cif = String::from_utf8_lossy(&cif_bytes);

    assert!(
        !cif.trim().is_empty(),
        "Expected mmCIF response to be non-empty for PDB id '{}'",
        pdb_id
    );

    let first_non_empty_line = cif
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    assert!(
        first_non_empty_line
            .to_ascii_lowercase()
            .starts_with("data_"),
        "Expected mmCIF first non-empty line to start with 'data_' (case-insensitive) for '{}', got '{}': URL {}",
        pdb_id,
        first_non_empty_line,
        url
    );

    assert!(
        first_non_empty_line
            .to_ascii_lowercase()
            .contains(&pdb_id.to_ascii_lowercase()),
        "Expected mmCIF data block name to include PDB id '{}' in first line '{}': URL {}",
        pdb_id,
        first_non_empty_line,
        url
    );

    assert!(
        cif.lines()
            .any(|line| line.trim_start().starts_with("loop_")),
        "Expected mmCIF content for '{}' to contain at least one 'loop_' line",
        pdb_id
    );

    assert!(
        cif.contains("_atom_site.label_atom_id")
            || cif.contains("_entity_poly.pdbx_seq_one_letter_code"),
        "Expected mmCIF content for '{}' to include key tags '_atom_site.label_atom_id' or '_entity_poly.pdbx_seq_one_letter_code'",
        pdb_id
    );

    assert!(
        cif_bytes.len() > 10 * 1024,
        "Expected mmCIF content for '{}' to exceed 10KB, got {} bytes",
        pdb_id,
        cif_bytes.len()
    );

    Ok(())
}

#[tokio::test]
async fn fetch_rcsb_pdb_basic_structure() -> Result<(), Box<dyn Error>> {
    let client = rcsb_client()?;
    let pdb_id = "4HHB";
    let url = rcsb_pdb_url(pdb_id);
    let pdb = fetch_text(&client, &url).await?;

    assert!(
        !pdb.trim().is_empty(),
        "Expected PDB response to be non-empty for PDB id '{}'",
        pdb_id
    );

    let first_non_empty_line = pdb
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    assert!(
        first_non_empty_line.starts_with("HEADER") || first_non_empty_line.starts_with("TITLE"),
        "Expected PDB first non-empty line to start with 'HEADER' or 'TITLE' for '{}', got '{}': URL {}",
        pdb_id,
        first_non_empty_line,
        url
    );

    assert!(
        pdb.contains("ATOM") || pdb.contains("HETATM"),
        "Expected PDB content for '{}' to include 'ATOM' or 'HETATM'",
        pdb_id
    );

    let atom_count = pdb
        .lines()
        .filter(|line| line.starts_with("ATOM") || line.starts_with("HETATM"))
        .take(10)
        .count();
    assert!(
        atom_count > 0,
        "Expected PDB content for '{}' to contain at least one line starting with 'ATOM' or 'HETATM'",
        pdb_id
    );

    assert!(
        pdb.lines()
            .any(|line| line.starts_with("END") || line.starts_with("ENDMDL")),
        "Expected PDB content for '{}' to contain an 'END' or 'ENDMDL' line",
        pdb_id
    );

    assert!(
        pdb.len() > 10 * 1024,
        "Expected PDB content for '{}' to exceed 10KB, got {} bytes",
        pdb_id,
        pdb.len()
    );

    Ok(())
}

#[tokio::test]
async fn fetch_rcsb_graphql_metadata() -> Result<(), Box<dyn Error>> {
    let client = rcsb_client()?;
    let pdb_id = "1A2B";
    let entry = fetch_rcsb_entry_metadata(&client, pdb_id).await?;

    assert_eq!(
        entry.rcsb_id.to_ascii_uppercase(),
        pdb_id,
        "Expected RCSB GraphQL entry_id to match requested id '{}', got '{}'",
        pdb_id,
        entry.rcsb_id
    );

    let title = entry
        .r#struct
        .as_ref()
        .map(|s| s.title.trim())
        .unwrap_or_default();
    assert!(
        !title.is_empty(),
        "Expected RCSB GraphQL title to be non-empty for entry '{}'",
        pdb_id
    );

    let molecular_weight = entry
        .rcsb_entry_info
        .as_ref()
        .and_then(|info| info.molecular_weight)
        .ok_or_else(|| {
            format!(
                "Expected molecular_weight field in RCSB GraphQL payload for entry '{}'",
                pdb_id
            )
        })?;
    assert!(
        molecular_weight > 0.0,
        "Expected molecular_weight > 0 for entry '{}', got {}",
        pdb_id,
        molecular_weight
    );

    Ok(())
}

#[tokio::test]
async fn rcsb_invalid_id_returns_error() {
    let client = rcsb_client().unwrap();
    let bad_id = "NOTREAL";

    let bad_cif_url = rcsb_cif_url(bad_id);
    let cif_result = fetch_bytes(&client, &bad_cif_url).await;
    assert!(
        cif_result.is_err(),
        "Expected mmCIF fetch to fail for invalid RCSB id '{}'",
        bad_id
    );
    if let Err(error) = cif_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad_id) || msg.contains("404") || msg.contains("Not Found"),
            "Expected mmCIF error to mention invalid id or 404-like status for '{}', got '{}'",
            bad_id,
            msg
        );
    }

    let bad_pdb_url = rcsb_pdb_url(bad_id);
    let pdb_result = fetch_text(&client, &bad_pdb_url).await;
    assert!(
        pdb_result.is_err(),
        "Expected PDB fetch to fail for invalid RCSB id '{}'",
        bad_id
    );
    if let Err(error) = pdb_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad_id) || msg.contains("404") || msg.contains("Not Found"),
            "Expected PDB error to mention invalid id or 404-like status for '{}', got '{}'",
            bad_id,
            msg
        );
    }

    let gql_result = fetch_rcsb_entry_metadata(&client, bad_id).await;
    assert!(
        gql_result.is_err(),
        "Expected GraphQL metadata fetch to fail for invalid RCSB id '{}'",
        bad_id
    );
    if let Err(error) = gql_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad_id) || msg.contains("404") || msg.contains("Not Found"),
            "Expected GraphQL error to mention invalid id or 404-like status for '{}', got '{}'",
            bad_id,
            msg
        );
    }
}
