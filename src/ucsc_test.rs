use flate2::read::GzDecoder;
use reqwest::Client;
use std::error::Error;
use std::io::Read;

fn ucsc_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (ucsc-integration-test)")
        .build()
}

fn ucsc_hg19_to_hg38_chain_url() -> &'static str {
    "https://hgdownload.soe.ucsc.edu/goldenPath/hg19/liftOver/hg19ToHg38.over.chain.gz"
}

fn ucsc_hg38_cytoband_txt_gz_url() -> &'static str {
    "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/database/cytoBand.txt.gz"
}

fn ucsc_hg38_gc5base_bigwig_url() -> &'static str {
    "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/bigZips/hg38.gc5Base.bw"
}

async fn fetch_bytes(client: &Client, url: &str) -> Result<bytes::Bytes, Box<dyn Error>> {
    let response = client.get(url).send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(
            format!("UCSC request failed for URL '{url}' with status {status}: {body}").into(),
        );
    }

    let bytes = response.bytes().await?;
    Ok(bytes)
}

async fn fetch_bytes_range(
    client: &Client,
    url: &str,
    range: &str,
) -> Result<bytes::Bytes, Box<dyn Error>> {
    let response = client.get(url).header("Range", range).send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(
            format!("UCSC request failed for URL '{url}' with status {status}: {body}").into(),
        );
    }

    let bytes = response.bytes().await?;
    Ok(bytes)
}

async fn fetch_and_gunzip_to_string(client: &Client, url: &str) -> Result<String, Box<dyn Error>> {
    let bytes = fetch_bytes(client, url).await?;
    let mut decoder = GzDecoder::new(&bytes[..]);
    let mut out = String::new();
    decoder.read_to_string(&mut out)?;
    Ok(out)
}

#[tokio::test]
async fn ucsc_hg19_to_hg38_chain_has_expected_structure() -> Result<(), Box<dyn Error>> {
    let client = ucsc_client()?;
    let chain_text = fetch_and_gunzip_to_string(&client, ucsc_hg19_to_hg38_chain_url()).await?;

    assert!(
        !chain_text.trim().is_empty(),
        "Expected UCSC hg19ToHg38 chain text to be non-empty"
    );
    assert!(
        chain_text.lines().any(|line| line.starts_with("chain ")),
        "Expected UCSC hg19ToHg38 chain file to contain at least one 'chain' header line"
    );
    assert!(
        chain_text.lines().any(|line| line.contains("chr13")),
        "Expected hg19ToHg38 chain file to contain at least one mapping line mentioning 'chr13'"
    );

    let line_count = chain_text.lines().count();
    assert!(
        line_count > 1000,
        "Expected hg19ToHg38 chain file to have many lines, got {}",
        line_count
    );

    let chain_header = chain_text
        .lines()
        .find(|line| line.starts_with("chain "))
        .expect("Expected at least one chain header line");
    let fields: Vec<_> = chain_header.split_whitespace().collect();
    assert!(
        fields.len() >= 7,
        "Expected chain header to have at least 7 whitespace-separated fields, got {}: '{}'",
        fields.len(),
        chain_header
    );

    Ok(())
}

#[tokio::test]
async fn ucsc_hg38_cytoband_table_has_chr13_rows() -> Result<(), Box<dyn Error>> {
    let client = ucsc_client()?;
    let cytoband_text =
        fetch_and_gunzip_to_string(&client, ucsc_hg38_cytoband_txt_gz_url()).await?;

    assert!(
        !cytoband_text.trim().is_empty(),
        "Expected UCSC cytoband table text to be non-empty"
    );
    assert!(
        cytoband_text.lines().any(|line| line.starts_with("chr")),
        "Expected cytoband table to contain lines beginning with 'chr'"
    );
    assert!(
        cytoband_text.lines().any(|line| line.starts_with("chr13")),
        "Expected cytoband table to contain at least one row for chr13"
    );

    let rows: Vec<_> = cytoband_text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect();

    for row in rows.iter().take(10) {
        let cols: Vec<_> = row.split('\t').collect();
        assert!(
            cols.len() >= 4,
            "Expected cytoband row to have at least 4 tab-separated columns, got {}: '{}'",
            cols.len(),
            row
        );

        let start: u64 = cols[1]
            .parse()
            .expect("Expected cytoband start to parse as u64");
        let end: u64 = cols[2]
            .parse()
            .expect("Expected cytoband end to parse as u64");

        assert!(
            start < end,
            "Expected cytoband interval start < end, got start={}, end={} in row '{}'",
            start,
            end,
            row
        );
    }

    Ok(())
}

#[tokio::test]
async fn ucsc_hg38_gc5base_bigwig_has_valid_magic_and_nontrivial_size() -> Result<(), Box<dyn Error>>
{
    let client = ucsc_client()?;
    let bytes = fetch_bytes_range(&client, ucsc_hg38_gc5base_bigwig_url(), "bytes=0-4095").await?;

    let len = bytes.len();
    assert!(
        len >= 4,
        "Expected bigWig file to be at least 4 bytes long, got {} bytes",
        len
    );

    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    assert_eq!(
        magic, 0x888F_FC26,
        "Expected bigWig magic 0x888FFC26, got 0x{:08X}",
        magic
    );

    Ok(())
}

#[tokio::test]
async fn ucsc_invalid_url_returns_error() {
    let client = ucsc_client().unwrap();
    let bad_url = "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/this_does_not_exist.xyz";

    let result = fetch_bytes(&client, bad_url).await;
    assert!(
        result.is_err(),
        "Expected fetch_bytes to return error for invalid UCSC URL '{}'",
        bad_url
    );

    if let Err(error) = result {
        let msg = error.to_string();
        assert!(
            msg.contains("this_does_not_exist") || msg.contains("404") || msg.contains("Not Found"),
            "Expected UCSC error message to mention invalid path or 404, got '{}'",
            msg
        );
    }
}
