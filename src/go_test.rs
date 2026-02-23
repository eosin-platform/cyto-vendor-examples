use reqwest::Client;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::OnceCell;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const GO_OBO_URL: &str = "https://ontologies.berkeleybop.org/go.obo";
const GO_OBO_PURL: &str = "https://purl.obolibrary.org/obo/go.obo";
static GO_OBO_CACHE: OnceCell<String> = OnceCell::const_new();

fn go_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (go-integration-test)")
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
        format!("GO request failed for URL '{url}': {error}").into()
    })?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let snippet = truncate_for_error(&text, 4096);

    if !status.is_success() {
        return Err(
            format!("GO request failed for URL '{url}' with status {status}: {snippet}").into(),
        );
    }

    Ok(text)
}

async fn fetch_go_obo_text(client: &Client) -> Result<String, AnyError> {
    match fetch_text(client, GO_OBO_URL).await {
        Ok(text) => Ok(text),
        Err(primary_error) => fetch_text(client, GO_OBO_PURL)
            .await
            .map_err(|fallback_error| {
                format!(
                    "Failed to fetch GO OBO from primary URL '{}' ({}) and fallback URL '{}' ({})",
                    GO_OBO_URL, primary_error, GO_OBO_PURL, fallback_error
                )
                .into()
            }),
    }
}

async fn cached_go_obo_text(client: &Client) -> Result<&'static String, AnyError> {
    GO_OBO_CACHE
        .get_or_try_init(|| async { fetch_go_obo_text(client).await })
        .await
}

fn is_header_line(line: &str) -> bool {
    line.starts_with("format-version:")
        || line.starts_with("data-version:")
        || line.starts_with("date:")
        || line.starts_with("ontology:")
}

#[derive(Debug)]
struct GoTerm<'a> {
    id: &'a str,
    name: &'a str,
    namespace: &'a str,
    is_a: Vec<&'a str>,
}

fn parse_go_terms(obo: &str) -> Vec<GoTerm<'_>> {
    let mut terms = Vec::new();

    for block in obo.split("\n[Term]").skip(1) {
        let mut id = "";
        let mut name = "";
        let mut namespace = "";
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
            if let Some(value) = line.strip_prefix("namespace:") {
                namespace = value.trim();
                continue;
            }
            if let Some(value) = line.strip_prefix("is_a:")
                && let Some(parent_id) = value.split_whitespace().next()
            {
                is_a.push(parent_id);
            }
        }

        if !id.is_empty() {
            terms.push(GoTerm {
                id,
                name,
                namespace,
                is_a,
            });
        }
    }

    terms
}

#[tokio::test]
async fn go_obo_contains_basic_header_and_known_terms() -> Result<(), AnyError> {
    let client = go_client()?;
    let obo_text = cached_go_obo_text(&client).await?;

    assert!(
        !obo_text.trim().is_empty(),
        "Expected GO OBO text fetched from GO to be non-empty",
    );
    assert!(
        obo_text.lines().any(is_header_line),
        "Expected GO OBO header to include at least one recognized header line"
    );
    assert!(
        obo_text
            .lines()
            .any(|line| line.starts_with("format-version:")),
        "Expected GO OBO header to contain 'format-version:'"
    );
    assert!(
        obo_text
            .lines()
            .any(|line| line.starts_with("data-version:")),
        "Expected GO OBO header to contain 'data-version:'"
    );
    assert!(
        obo_text
            .lines()
            .any(|line| line.to_ascii_lowercase().starts_with("ontology: go")),
        "Expected GO OBO header to contain 'ontology: go'"
    );

    let terms = parse_go_terms(obo_text);

    assert!(
        terms.len() > 10_000,
        "Expected GO OBO parser to extract many terms (>10,000), got {}",
        terms.len()
    );

    let root_bp = terms
        .iter()
        .find(|term| term.id == "GO:0008150")
        .ok_or_else(|| {
            "Expected GO root biological process term GO:0008150 to be present".to_string()
        })?;

    assert!(
        !root_bp.name.trim().is_empty(),
        "Expected GO:0008150 to have a non-empty name"
    );
    assert!(
        root_bp.namespace.eq_ignore_ascii_case("biological_process"),
        "Expected GO:0008150 namespace to be 'biological_process', got '{}'",
        root_bp.namespace
    );

    Ok(())
}

#[tokio::test]
async fn go_obo_contains_all_core_namespaces() -> Result<(), AnyError> {
    let client = go_client()?;
    let obo_text = cached_go_obo_text(&client).await?;
    let terms = parse_go_terms(obo_text);

    let namespaces: HashSet<String> = terms
        .iter()
        .map(|term| term.namespace.to_ascii_lowercase())
        .filter(|ns| !ns.trim().is_empty())
        .collect();

    assert!(
        namespaces.contains("biological_process"),
        "Expected GO OBO terms to include namespace 'biological_process', got {:?}",
        namespaces
    );
    assert!(
        namespaces.contains("molecular_function"),
        "Expected GO OBO terms to include namespace 'molecular_function', got {:?}",
        namespaces
    );
    assert!(
        namespaces.contains("cellular_component"),
        "Expected GO OBO terms to include namespace 'cellular_component', got {:?}",
        namespaces
    );

    Ok(())
}

#[tokio::test]
async fn go_obo_has_basic_dag_structure_for_root_terms() -> Result<(), AnyError> {
    let client = go_client()?;
    let obo_text = cached_go_obo_text(&client).await?;
    let terms = parse_go_terms(obo_text);

    let expected_roots = [
        ("GO:0008150", "biological_process"),
        ("GO:0003674", "molecular_function"),
        ("GO:0005575", "cellular_component"),
    ];

    for (go_id, expected_namespace) in expected_roots {
        let term = terms
            .iter()
            .find(|term| term.id == go_id)
            .ok_or_else(|| format!("Expected GO root term '{}' to be present", go_id))?;

        assert!(
            !term.name.trim().is_empty(),
            "Expected GO root term '{}' to have a non-empty name",
            go_id
        );
        assert!(
            term.namespace.eq_ignore_ascii_case(expected_namespace),
            "Expected GO root term '{}' namespace to be '{}', got '{}'",
            go_id,
            expected_namespace,
            term.namespace
        );
    }

    let non_root_bp_term = terms
        .iter()
        .find(|term| {
            term.namespace.eq_ignore_ascii_case("biological_process")
                && term.id != "GO:0008150"
                && !term.is_a.is_empty()
        })
        .ok_or_else(|| {
            "Expected to find a non-root biological_process term with at least one is_a parent"
                .to_string()
        })?;

    assert!(
        non_root_bp_term
            .is_a
            .iter()
            .any(|parent_id| parent_id.starts_with("GO:")),
        "Expected non-root biological_process term '{}' to have at least one GO: is_a parent, got {:?}",
        non_root_bp_term.id,
        non_root_bp_term.is_a
    );

    Ok(())
}

#[tokio::test]
async fn go_obo_invalid_url_returns_error() -> Result<(), AnyError> {
    let client = go_client()?;
    let bad_url = "https://ontologies.berkeleybop.org/go_DOES_NOT_EXIST.obo";
    let result = fetch_text(&client, bad_url).await;

    assert!(
        result.is_err(),
        "Expected GO OBO fetch to fail for invalid URL '{}'",
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
            "Expected GO invalid-URL error message to mention a 404/not-found style error, got '{}'",
            msg
        );
    }

    Ok(())
}
