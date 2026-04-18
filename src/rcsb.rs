use anyhow::{Result, anyhow, ensure};
use bytes::Bytes;
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;

const DEFAULT_USER_AGENT: &str = "cyto-vendor-examples/0.1 (integration-test)";
const RCSB_GRAPHQL_URL: &str = "https://data.rcsb.org/graphql";
const RCSB_SEARCH_URL: &str = "https://search.rcsb.org/rcsbsearch/v2/query";
const RCSB_FILES_BASE_URL: &str = "https://files.rcsb.org/download";

pub struct RcsbClient {
    client: Client,
    graphql_url: String,
    search_url: String,
    files_base_url: String,
}

impl RcsbClient {
    pub fn new() -> Result<Self> {
        Self::with_endpoints(RCSB_GRAPHQL_URL, RCSB_SEARCH_URL, RCSB_FILES_BASE_URL)
    }

    pub fn with_endpoints(
        graphql_url: &str,
        search_url: &str,
        files_base_url: &str,
    ) -> Result<Self> {
        let client = Client::builder().user_agent(DEFAULT_USER_AGENT).build()?;
        Ok(Self {
            client,
            graphql_url: graphql_url.trim_end_matches('/').to_string(),
            search_url: search_url.trim_end_matches('/').to_string(),
            files_base_url: files_base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn download_cif_bytes(&self, pdb_id: &str) -> Result<Bytes> {
        let url = self.cif_url(pdb_id);
        self.fetch_bytes(&url)
            .await
            .map_err(|err| anyhow!("Failed to download mmCIF for '{pdb_id}' from '{url}': {err}"))
    }

    pub async fn download_cif_text(&self, pdb_id: &str) -> Result<String> {
        let url = self.cif_url(pdb_id);
        self.fetch_text(&url)
            .await
            .map_err(|err| anyhow!("Failed to download mmCIF for '{pdb_id}' from '{url}': {err}"))
    }

    pub async fn download_pdb_bytes(&self, pdb_id: &str) -> Result<Bytes> {
        let url = self.pdb_url(pdb_id);
        self.fetch_bytes(&url)
            .await
            .map_err(|err| anyhow!("Failed to download PDB for '{pdb_id}' from '{url}': {err}"))
    }

    pub async fn download_pdb_text(&self, pdb_id: &str) -> Result<String> {
        let url = self.pdb_url(pdb_id);
        self.fetch_text(&url)
            .await
            .map_err(|err| anyhow!("Failed to download PDB for '{pdb_id}' from '{url}': {err}"))
    }

    /// Searches RCSB for matching entries using the official RCSB Search API.
    ///
    /// This is a general-purpose "protein structure" search entry point: pass a PDB id,
    /// protein name, gene, UniProt accession, author name, etc. (free-text).
    pub async fn search_entries_text(
        &self,
        term: &str,
        offset: usize,
        limit: usize,
    ) -> Result<RcsbSearchResult> {
        ensure!(limit > 0, "limit must be > 0");

        let body = json!({
            "query": {
                "type": "terminal",
                "service": "full_text",
                "parameters": {
                    "value": term,
                }
            },
            "return_type": "entry",
            "request_options": {
                "paginate": {
                    "start": offset,
                    "rows": limit,
                }
            }
        });

        let response: RcsbSearchResponse = self
            .post_json(&self.search_url, &body)
            .await
            .map_err(|err| anyhow!("Failed to search RCSB entries for term '{term}': {err}"))?;

        Ok(RcsbSearchResult {
            items: response
                .result_set
                .into_iter()
                .map(|item| item.identifier())
                .collect(),
            full_count: response.total_count,
            limit,
            offset,
        })
    }

    pub async fn fetch_entry_metadata(&self, pdb_id: &str) -> Result<RcsbEntry> {
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

        let data: RcsbGraphqlData = self
            .graphql_query(RCSB_ENTRY_QUERY, json!({ "id": pdb_id }))
            .await
            .map_err(|err| anyhow!("Failed to fetch RCSB metadata for '{pdb_id}': {err}"))?;

        data.entry.ok_or_else(|| {
            anyhow!("RCSB GraphQL returned null entry payload for entry id '{pdb_id}'")
        })
    }

    /// Fetches a "maximal" metadata payload for an entry and prints the raw JSON.
    ///
    /// This intentionally returns the raw JSON so we can iterate on the Rust types later.
    pub async fn fetch_entry_metadata_full(&self, pdb_id: &str) -> Result<serde_json::Value> {
        const RCSB_ENTRY_QUERY_FULL: &str = r#"
query ($id: String!) {
    entry(entry_id: $id) {
        rcsb_id
        rcsb_accession_info {
            deposit_date
            has_released_experimental_data
            initial_release_date
            major_revision
            minor_revision
            revision_date
            status_code
        }
        rcsb_entry_container_identifiers {
            assembly_ids
            branched_entity_ids
            emdb_ids
            entity_ids
            entry_id
            model_ids
            non_polymer_entity_ids
            polymer_entity_ids
            pubmed_id
            rcsb_id
            related_emdb_ids
            water_entity_ids
        }
        rcsb_entry_info {
            assembly_count
            branched_entity_count
            branched_molecular_weight_maximum
            branched_molecular_weight_minimum
            cis_peptide_count
            deposited_atom_count
            deposited_deuterated_water_count
            deposited_hydrogen_atom_count
            deposited_model_count
            deposited_modeled_polymer_monomer_count
            deposited_nonpolymer_entity_instance_count
            deposited_polymer_entity_instance_count
            deposited_polymer_monomer_count
            deposited_solvent_atom_count
            deposited_unmodeled_polymer_monomer_count
            diffrn_radiation_wavelength_maximum
            diffrn_radiation_wavelength_minimum
            diffrn_resolution_high {
                provenance_source
                value
            }
            disulfide_bond_count
            entity_count
            experimental_method
            experimental_method_count
            ihm_multi_scale_flag
            ihm_multi_state_flag
            ihm_ordered_state_flag
            ihm_structure_description
            inter_mol_covalent_bond_count
            inter_mol_metalic_bond_count
            molecular_weight
            na_polymer_entity_types
            ndb_struct_conf_na_feature_combined
            nonpolymer_bound_components
            nonpolymer_entity_count
            nonpolymer_molecular_weight_maximum
            nonpolymer_molecular_weight_minimum
            polymer_composition
            polymer_entity_count
            polymer_entity_count_DNA
            polymer_entity_count_RNA
            polymer_entity_count_nucleic_acid
            polymer_entity_count_nucleic_acid_hybrid
            polymer_entity_count_protein
            polymer_entity_taxonomy_count
            polymer_molecular_weight_maximum
            polymer_molecular_weight_minimum
            polymer_monomer_count_maximum
            polymer_monomer_count_minimum
            representative_model
            resolution_combined
            selected_polymer_entity_types
            software_programs_combined
            solvent_entity_count
            structure_determination_methodology
            structure_determination_methodology_priority
        }
        struct {
            pdbx_CASP_flag
            pdbx_descriptor
            pdbx_model_details
            pdbx_model_type_details
            title
        }
        struct_keywords {
            pdbx_keywords
            text
        }
        pdbx_database_status {
            SG_entry
            deposit_site
            methods_development_category
            pdb_format_compatible
            process_site
            recvd_initial_deposition_date
            status_code
            status_code_cs
            status_code_mr
            status_code_sf
        }
        exptl {
            crystals_number
            details
            method
            method_details
        }
        cell {
            Z_PDB
            angle_alpha
            angle_beta
            angle_gamma
            formula_units_Z
            length_a
            length_b
            length_c
            pdbx_unique_axis
            volume
        }
        symmetry {
            Int_Tables_number
            cell_setting
            pdbx_full_space_group_name_H_M
            space_group_name_H_M
            space_group_name_Hall
        }
        audit_author {
            identifier_ORCID
            name
            pdbx_ordinal
        }
        citation {
            book_id_ISBN
            book_publisher
            book_publisher_city
            book_title
            coordinate_linkage
            country
            id
            journal_abbrev
            journal_full
            journal_id_ASTM
            journal_id_CSD
            journal_id_ISSN
            journal_issue
            journal_volume
            language
            page_first
            page_last
            pdbx_database_id_DOI
            pdbx_database_id_PubMed
            rcsb_authors
            rcsb_is_primary
            rcsb_journal_abbrev
            title
            unpublished_flag
            year
        }
        software {
            citation_id
            classification
            contact_author
            contact_author_email
            date
            description
            language
            location
            name
            os
            pdbx_ordinal
            type
            version
        }
        refine {
            B_iso_max
            B_iso_mean
            B_iso_min
            aniso_B_1_1
            aniso_B_1_2
            aniso_B_1_3
            aniso_B_2_2
            aniso_B_2_3
            aniso_B_3_3
            correlation_coeff_Fo_to_Fc
            correlation_coeff_Fo_to_Fc_free
            details
            ls_R_factor_R_free
            ls_R_factor_R_free_error
            ls_R_factor_R_free_error_details
            ls_R_factor_R_work
            ls_R_factor_all
            ls_R_factor_obs
            ls_d_res_high
            ls_d_res_low
            ls_matrix_type
            ls_number_parameters
            ls_number_reflns_R_free
            ls_number_reflns_R_work
            ls_number_reflns_all
            ls_number_reflns_obs
            ls_number_restraints
            ls_percent_reflns_R_free
            ls_percent_reflns_obs
            ls_redundancy_reflns_all
            ls_redundancy_reflns_obs
            ls_wR_factor_R_free
            ls_wR_factor_R_work
            occupancy_max
            occupancy_min
            overall_FOM_free_R_set
            overall_FOM_work_R_set
            overall_SU_B
            overall_SU_ML
            overall_SU_R_Cruickshank_DPI
            overall_SU_R_free
            pdbx_R_Free_selection_details
            pdbx_TLS_residual_ADP_flag
            pdbx_average_fsc_free
            pdbx_average_fsc_overall
            pdbx_average_fsc_work
            pdbx_data_cutoff_high_absF
            pdbx_data_cutoff_high_rms_absF
            pdbx_data_cutoff_low_absF
            pdbx_diffrn_id
            pdbx_isotropic_thermal_model
            pdbx_ls_cross_valid_method
            pdbx_ls_sigma_F
            pdbx_ls_sigma_Fsqd
            pdbx_ls_sigma_I
            pdbx_method_to_determine_struct
            pdbx_overall_ESU_R
            pdbx_overall_ESU_R_Free
            pdbx_overall_SU_R_Blow_DPI
            pdbx_overall_SU_R_free_Blow_DPI
        }
        reflns {
            B_iso_Wilson_estimate
            R_free_details
            Rmerge_F_all
            Rmerge_F_obs
            d_resolution_low
            d_resolution_high
            data_reduction_details
            data_reduction_method
            details
            limit_h_max
            limit_h_min
            limit_k_max
            limit_k_min
            limit_l_max
            limit_l_min
            number_all
            number_obs
            observed_criterion
            observed_criterion_F_max
            observed_criterion_F_min
            observed_criterion_I_max
            observed_criterion_I_min
            observed_criterion_sigma_F
            observed_criterion_sigma_I
            pdbx_CC_half
            pdbx_R_split
            pdbx_Rmerge_I_obs
            pdbx_Rpim_I_all
            pdbx_Rrim_I_all
            pdbx_Rsym_value
            pdbx_chi_squared
            pdbx_diffrn_id
            pdbx_netI_over_av_sigmaI
            pdbx_netI_over_sigmaI
            pdbx_number_measured_all
            pdbx_ordinal
            pdbx_redundancy
            pdbx_scaling_rejects
            percent_possible_obs
            phase_calculation_details
        }
        polymer_entities {
            rcsb_id
        }
        nonpolymer_entities {
            rcsb_id
        }
        branched_entities {
            rcsb_id
        }
        assemblies {
            rcsb_id
        }
    }
}
"#;

        let data: serde_json::Value = self
            .graphql_query(RCSB_ENTRY_QUERY_FULL, json!({ "id": pdb_id }))
            .await
            .map_err(|err| anyhow!("Failed to fetch RCSB full metadata for '{pdb_id}': {err}"))?;

        Ok(data)
    }

    fn cif_url(&self, pdb_id: &str) -> String {
        format!("{}/{pdb_id}.cif", self.files_base_url)
    }

    fn pdb_url(&self, pdb_id: &str) -> String {
        format!("{}/{pdb_id}.pdb", self.files_base_url)
    }

    async fn graphql_query<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T> {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let response: GraphqlEnvelope<T> = self
            .post_json(&self.graphql_url, &body)
            .await
            .map_err(|err| anyhow!("RCSB GraphQL request failed: {err}"))?;

        if !response.errors.is_empty() {
            let combined = response
                .errors
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(anyhow!("RCSB GraphQL returned errors: {combined}"));
        }

        response.data.ok_or_else(|| {
            anyhow!("RCSB GraphQL response contained no data payload for query: {query}")
        })
    }

    async fn fetch_text(&self, url: &str) -> Result<String> {
        let bytes = self.fetch_bytes(url).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    async fn fetch_bytes(&self, url: &str) -> Result<Bytes> {
        let response = self.client.get(url).send().await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "RCSB request failed for URL '{url}' with status {status}: {body}"
            ));
        }

        Ok(response.bytes().await?)
    }

    async fn post_json<T: DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let response = self.client.post(url).json(body).send().await?;
        let status = response.status();

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "RCSB request failed for URL '{url}' with status {status}: {text}"
            ));
        }

        Ok(response.json::<T>().await?)
    }
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

#[derive(Debug, Deserialize)]
struct RcsbGraphqlData {
    entry: Option<RcsbEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RcsbEntry {
    pub rcsb_id: String,
    #[serde(default)]
    pub r#struct: Option<RcsbStruct>,
    #[serde(default)]
    pub rcsb_entry_info: Option<RcsbEntryInfo>,
}

#[derive(Debug, Deserialize)]
pub struct RcsbStruct {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct RcsbEntryInfo {
    pub molecular_weight: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RcsbSearchResponse {
    total_count: usize,
    #[serde(default)]
    result_set: Vec<RcsbSearchResultItem>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RcsbSearchResultItem {
    Identifier(String),
    Hit(RcsbSearchHit),
}

impl RcsbSearchResultItem {
    fn identifier(self) -> String {
        match self {
            RcsbSearchResultItem::Identifier(id) => id,
            RcsbSearchResultItem::Hit(hit) => hit.identifier,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RcsbSearchHit {
    identifier: String,
    #[allow(dead_code)]
    score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RcsbSearchResult {
    pub items: Vec<String>,
    pub full_count: usize,
    pub limit: usize,
    pub offset: usize,
}
