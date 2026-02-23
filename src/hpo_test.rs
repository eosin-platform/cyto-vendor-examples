use reqwest::Client;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::OnceCell;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const HPO_OBO_URL: &str = "https://ontologies.berkeleybop.org/hp.obo";
const HPO_OBO_PURL: &str = "https://purl.obolibrary.org/obo/hp.obo";
static HPO_OBO_CACHE: OnceCell<String> = OnceCell::const_new();

fn hpo_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (hpo-integration-test)")
        .timeout(Duration::from_secs(20))
        .build()?;
    Ok(client)
}

fn truncate_for_error(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_owned();
    }
    let cutoff = text
        .char_indices()
        .nth(max_len)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| text.len());
    let snippet = &text[..cutoff];
    format!("{snippet}...[truncated]")
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, AnyError> {
    let response = client.get(url).send().await.map_err(|error| -> AnyError {
        format!("HPO request failed for URL '{url}': {error}").into()
    })?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let snippet = truncate_for_error(&text, 4096);

    if !status.is_success() {
        return Err(
            format!("HPO request failed for URL '{url}' with status {status}: {snippet}").into(),
        );
    }

    Ok(text)
}

async fn fetch_hpo_obo_text(client: &Client) -> Result<String, AnyError> {
    match fetch_text(client, HPO_OBO_URL).await {
        Ok(text) => Ok(text),
        Err(primary_error) => fetch_text(client, HPO_OBO_PURL)
            .await
            .map_err(|fallback_error| {
                format!(
                    "Failed to fetch HPO OBO from primary URL '{}' ({}) and fallback URL '{}' ({})",
                    HPO_OBO_URL, primary_error, HPO_OBO_PURL, fallback_error
                )
                .into()
            }),
    }
}

async fn cached_hpo_obo_text(client: &Client) -> Result<&'static String, AnyError> {
    HPO_OBO_CACHE
        .get_or_try_init(|| async { fetch_hpo_obo_text(client).await })
        .await
}

fn is_header_line(line: &str) -> bool {
    line.starts_with("format-version:")
        || line.starts_with("data-version:")
        || line.starts_with("date:")
        || line.starts_with("ontology:")
}

fn obo_header_lines(obo: &str) -> Vec<&str> {
    obo.lines()
        .take_while(|line| !line.trim_start().starts_with("[Term]"))
        .collect()
}

#[derive(Debug)]
struct HpoTerm<'a> {
    id: &'a str,
    name: &'a str,
    is_a: Vec<&'a str>,
}

fn parse_hpo_terms(obo: &str) -> Vec<HpoTerm<'_>> {
    let mut terms = Vec::new();

    for block in obo.split("\n[Term]").skip(1) {
        let mut id = "";
        let mut name = "";
        let mut is_a = Vec::new();

        for raw_line in block.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') {
                break;
            }

            if let Some(value) = line.strip_prefix("id:") {
                id = value.trim();
                continue;
            }
            if let Some(value) = line.strip_prefix("name:") {
                name = value.trim();
                continue;
            }
            if let Some(value) = line.strip_prefix("is_a:") {
                if let Some(parent_id) = value.trim().split_whitespace().next() {
                    is_a.push(parent_id);
                }
            }
        }

        if !id.is_empty() {
            terms.push(HpoTerm { id, name, is_a });
        }
    }

    terms
}

#[tokio::test]
async fn hpo_obo_contains_basic_header_and_known_terms() -> Result<(), AnyError> {
    let client = hpo_client()?;
    let obo_text = cached_hpo_obo_text(&client).await?;

    assert!(
        !obo_text.trim().is_empty(),
        "Expected HPO OBO text fetched from '{}' or fallback '{}' to be non-empty",
        HPO_OBO_URL,
        HPO_OBO_PURL
    );
    let header_lines = obo_header_lines(obo_text);
    assert!(
        header_lines.iter().any(|line| is_header_line(line)),
        "Expected HPO OBO header to include at least one recognized header line"
    );
    assert!(
        header_lines
            .iter()
            .any(|line| line.starts_with("format-version:")),
        "Expected HPO OBO header to contain 'format-version:'"
    );
    assert!(
        header_lines
            .iter()
            .any(|line| line.starts_with("data-version:")),
        "Expected HPO OBO header to contain 'data-version:'"
    );
    assert!(
        header_lines
            .iter()
            .any(|line| line.to_ascii_lowercase().starts_with("ontology: hp")),
        "Expected HPO OBO header to contain 'ontology: hp'"
    );

    let terms = parse_hpo_terms(obo_text);
    assert!(
        terms.len() > 5_000,
        "Expected HPO OBO parser to extract many terms (>5,000), got {}",
        terms.len()
    );

    let global_root = terms
        .iter()
        .find(|term| term.id == "HP:0000001")
        .ok_or_else(|| "Expected HPO global root term HP:0000001 to be present".to_string())?;

    assert!(
        !global_root.name.trim().is_empty(),
        "Expected HP:0000001 to have a non-empty name"
    );

    let root = terms
        .iter()
        .find(|term| term.id == "HP:0000118")
        .ok_or_else(|| "Expected HPO root term HP:0000118 to be present".to_string())?;

    assert!(
        !root.name.trim().is_empty(),
        "Expected HP:0000118 to have a non-empty name"
    );
    assert!(
        root.name
            .to_ascii_lowercase()
            .contains("phenotypic abnormality"),
        "Expected HP:0000118 name to contain 'Phenotypic abnormality', got '{}'",
        root.name
    );

    Ok(())
}

#[tokio::test]
async fn hpo_obo_contains_some_nonroot_term_with_parent() -> Result<(), AnyError> {
    let client = hpo_client()?;
    let obo_text = cached_hpo_obo_text(&client).await?;
    let terms = parse_hpo_terms(obo_text);

    let non_root_term = terms
        .iter()
        .find(|term| term.id != "HP:0000118" && !term.is_a.is_empty())
        .ok_or_else(|| {
            "Expected to find a non-root HPO term with at least one is_a parent".to_string()
        })?;

    assert!(
        !non_root_term.name.trim().is_empty(),
        "Expected non-root HPO term '{}' to have a non-empty name",
        non_root_term.id
    );
    assert!(
        non_root_term
            .is_a
            .iter()
            .any(|parent_id| parent_id.starts_with("HP:")),
        "Expected non-root HPO term '{}' to have at least one HP: parent, got {:?}",
        non_root_term.id,
        non_root_term.is_a
    );

    Ok(())
}

#[tokio::test]
async fn hpo_obo_has_multiple_top_level_branches() -> Result<(), AnyError> {
    let client = hpo_client()?;
    let obo_text = cached_hpo_obo_text(&client).await?;
    let terms = parse_hpo_terms(obo_text);

    let global_root = terms
        .iter()
        .find(|term| term.id == "HP:0000001")
        .ok_or_else(|| "Expected HPO global root term HP:0000001 to be present".to_string())?;

    assert!(
        !global_root.name.trim().is_empty(),
        "Expected HP:0000001 to have a non-empty name"
    );

    let top_level_children: HashSet<String> = terms
        .iter()
        .filter(|term| term.is_a.iter().any(|parent_id| *parent_id == "HP:0000001"))
        .map(|term| term.id.to_string())
        .collect();

    assert!(
        top_level_children.len() >= 2,
        "Expected HPO root (HP:0000001) to have at least 2 direct child branches, found {}",
        top_level_children.len()
    );
    assert!(
        top_level_children
            .iter()
            .any(|child_id| child_id.starts_with("HP:00")),
        "Expected at least one top-level HPO child ID under HP:0000001 to start with 'HP:00', got {:?}",
        top_level_children
    );

    Ok(())
}

#[tokio::test]
async fn hpo_obo_invalid_url_returns_error() -> Result<(), AnyError> {
    let client = hpo_client()?;
    let bad_url = "https://ontologies.berkeleybop.org/hp_DOES_NOT_EXIST.obo";
    let result = fetch_text(&client, bad_url).await;

    assert!(
        result.is_err(),
        "Expected HPO OBO fetch to fail for invalid URL '{}'",
        bad_url
    );

    if let Err(error) = result {
        let msg = error.to_string().to_ascii_lowercase();
        assert!(
            msg.contains("404")
                || msg.contains("not found")
                || msg.contains("failed")
                || msg.contains("could not")
                || msg.contains("error sending request")
                || msg.contains("certificate"),
            "Expected HPO invalid-URL error message to mention a 404/not-found style error, got '{}'",
            msg
        );
    }

    Ok(())
}
