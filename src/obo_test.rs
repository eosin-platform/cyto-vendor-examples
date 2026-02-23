use reqwest::Client;
use std::error::Error;

fn obo_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (obo-integration-test)")
        .build()
}

fn go_obo_url() -> &'static str {
    "https://purl.obolibrary.org/obo/go.obo"
}

fn hpo_obo_url() -> &'static str {
    "https://purl.obolibrary.org/obo/hp.obo"
}

async fn fetch_obo_text(client: &Client, url: &str) -> Result<String, Box<dyn Error>> {
    let response = client.get(url).send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(
            format!("OBO request failed for URL '{url}' with status {status}: {body}").into(),
        );
    }

    let text = response.text().await?;
    Ok(text)
}

#[derive(Debug)]
struct OboTerm<'a> {
    id: &'a str,
    name: &'a str,
    is_a: Vec<&'a str>,
}

fn parse_obo_terms<'a>(text: &'a str) -> Vec<OboTerm<'a>> {
    let mut terms = Vec::new();

    for stanza in text.split("\n\n") {
        let stanza = stanza.trim();
        if !stanza.starts_with("[Term]") {
            continue;
        }

        let mut id: Option<&'a str> = None;
        let mut name: Option<&'a str> = None;
        let mut is_a = Vec::new();
        let mut is_obsolete = false;

        for line in stanza.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("id: ") {
                id = Some(rest.trim());
            } else if let Some(rest) = line.strip_prefix("name: ") {
                name = Some(rest.trim());
            } else if let Some(rest) = line.strip_prefix("is_a: ") {
                let parent = rest.split('!').next().unwrap_or("").trim();
                if !parent.is_empty() {
                    is_a.push(parent);
                }
            } else if line == "is_obsolete: true" {
                is_obsolete = true;
            }
        }

        if !is_obsolete && let (Some(id), Some(name)) = (id, name) {
            terms.push(OboTerm { id, name, is_a });
        }
    }

    terms
}

fn find_term<'a>(terms: &'a [OboTerm<'a>], id: &str) -> Option<&'a OboTerm<'a>> {
    terms.iter().find(|term| term.id == id)
}

#[tokio::test]
async fn go_obo_contains_basic_header_and_known_terms() -> Result<(), Box<dyn Error>> {
    let client = obo_client()?;
    let text = fetch_obo_text(&client, go_obo_url()).await?;

    assert!(
        text.lines().any(|line| line.starts_with("format-version:")),
        "Expected GO OBO to contain a 'format-version:' header"
    );
    assert!(
        text.lines().any(|line| {
            line.to_ascii_lowercase().contains("ontology: go") || line.contains("data-version:")
        }),
        "Expected GO OBO to contain an 'ontology: go' or 'data-version:' header"
    );

    let terms = parse_obo_terms(&text);
    assert!(
        !terms.is_empty(),
        "Expected GO OBO to contain at least one [Term] stanza"
    );

    let bp = find_term(&terms, "GO:0008150").expect("Expected GO:0008150 in GO OBO");
    assert!(
        !bp.name.trim().is_empty(),
        "Expected GO:0008150 to have a non-empty name"
    );
    let bp_name = bp.name.to_ascii_lowercase();
    assert!(
        bp_name.contains("biological") && bp_name.contains("process"),
        "Expected GO:0008150 name to mention 'biological process', got '{}'",
        bp.name
    );

    let nucleus = find_term(&terms, "GO:0005634").expect("Expected GO:0005634 in GO OBO");
    assert!(
        !nucleus.name.trim().is_empty(),
        "Expected GO:0005634 to have a non-empty name"
    );
    let nucleus_name = nucleus.name.to_ascii_lowercase();
    assert!(
        nucleus_name.contains("nucleus"),
        "Expected GO:0005634 name to mention 'nucleus', got '{}'",
        nucleus.name
    );

    assert!(
        !bp.is_a.is_empty() || !nucleus.is_a.is_empty(),
        "Expected at least one known GO term to have a non-empty is_a parent list"
    );
    assert!(
        bp.is_a.iter().any(|parent| parent.starts_with("GO:"))
            || nucleus.is_a.iter().any(|parent| parent.starts_with("GO:")),
        "Expected at least one parent of GO:0008150 or GO:0005634 to look like a GO:... id, got parents: bp={:?}, nucleus={:?}",
        bp.is_a,
        nucleus.is_a
    );

    Ok(())
}

#[tokio::test]
async fn hpo_obo_contains_basic_header_and_known_terms() -> Result<(), Box<dyn Error>> {
    let client = obo_client()?;
    let text = fetch_obo_text(&client, hpo_obo_url()).await?;

    assert!(
        text.lines().any(|line| line.starts_with("format-version:")),
        "Expected HPO OBO to contain a 'format-version:' header"
    );
    assert!(
        text.lines().any(|line| {
            line.to_ascii_lowercase().contains("ontology: hp") || line.contains("data-version:")
        }),
        "Expected HPO OBO to contain an 'ontology: hp' or 'data-version:' header"
    );
    assert!(
        text.lines().any(|line| line.starts_with("idspace:")),
        "Expected HPO OBO to contain at least one idspace declaration"
    );
    assert!(
        text.lines().any(|line| line.starts_with("id: HP:")),
        "Expected HPO OBO to contain HP term identifiers (id: HP:...)"
    );

    let terms = parse_obo_terms(&text);
    assert!(
        !terms.is_empty(),
        "Expected HPO OBO to contain at least one [Term] stanza"
    );

    let seizure = find_term(&terms, "HP:0001250").expect("Expected HP:0001250 in HPO OBO");
    assert!(
        !seizure.name.trim().is_empty(),
        "Expected HP:0001250 to have a non-empty name"
    );
    let seizure_name = seizure.name.to_ascii_lowercase();
    assert!(
        seizure_name.contains("seiz"),
        "Expected HP:0001250 name to mention seizure-related text, got '{}'",
        seizure.name
    );
    assert!(
        !seizure.is_a.is_empty(),
        "Expected HP:0001250 to have at least one is_a parent"
    );
    assert!(
        seizure.is_a.iter().any(|parent| parent.starts_with("HP:")),
        "Expected HP:0001250 to have at least one HP:... parent, got parents: {:?}",
        seizure.is_a
    );

    Ok(())
}

#[tokio::test]
async fn obo_invalid_url_returns_error() {
    let client = obo_client().unwrap();
    let bad_url = "https://purl.obolibrary.org/obo/this_does_not_exist.obo";

    let result = fetch_obo_text(&client, bad_url).await;
    assert!(
        result.is_err(),
        "Expected fetch_obo_text to return an error for invalid OBO URL '{}'",
        bad_url
    );
}
