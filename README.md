# **Vendor Examples — Cyto External Data Integration Tests**
[![CI](https://github.com/eosin-platform/cyto-vendor-examples/actions/workflows/ci.yaml/badge.svg?branch=master)](https://github.com/eosin-platform/cyto-vendor-examples/actions/workflows/ci.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../LICENSE-MIT)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE-APACHE-2.0)

**Related blog post:** https://thavlik.dev/blog/2026-02-23/cyto-vendor-examples

This crate contains **integration tests** and **usage examples** for all external data vendors that Cyto interacts with.
The goal is **maximum reproducibility** and **guaranteed correctness** across the entire Cyto ecosystem: WSI, genomics, proteomics, ontologies, structures, and cancer datasets.

Every vendor test demonstrates:

* Expected API behavior
* Known stable identifiers (e.g., BRCA2, ENST00000380152)
* HTTP patterns (REST, FTP, GraphQL, binary formats)
* Decompression, parsing, and edge-case handling
* Resilience to vendor quirks (timeouts, alternate formats, slow endpoints)

This repo acts as both:

1. **A reference implementation** for Cyto vendor fetches
2. **A test suite** that validates correctness against live upstream APIs
3. **A coverage roadmap**, showing which vendors have been integrated and which remain

---

# **Why This Exists**

The world of scientific data is sprawling and inconsistent.
Cyto aims to make it feel like Docker Hub for biology — predictable, clean, typed, and uniform.

But underneath, each vendor has:

* Different error semantics
* Different throttling behavior
* Different naming conventions
* Different file formats
* Strong or weak uptime guarantees

This repository encodes all that nuance in executable tests.

---

# **Vendor Support Matrix**

**Tier Definitions**

* **Tier 0 — Canonical / Source of Truth**
  Government or foundational institutions; must be rock-solid.

* **Tier 1 — High-Authority / Regulated**
  Extremely important and high quality; not canonical but close.

* **Tier 2 — Secondary Databases**
  Strong integrators, cross-linkers, utilities; occasional gaps.

* **Tier 3 — Niche / Research-Grade**
  Useful but not essential or with uptime/licensing issues.

**Tested? Column**

* ✔️ = Implemented test suite exists
* ❌ = Planned, but not implemented yet (roadmap)
* ➖ = Not applicable (non-API bulk download or incompatible licensing)

---

# **📊 Comprehensive Vendor Table**

Below is the full vendor matrix with **Tier** and **Tested?** status. Vendors that lack an API but can be downloaded in full are not included here (e.g. TargetScan, miRBase)
| Vendor                                 | Domain                                  | API / Access            | Tier    | Tested? | Notes                                                                      |
| --------------------------------------- | ----------------------------------------- | ------------------------ | ------- | ------- | -------------------------------------------------------------------------- |
| **NCBI (Entrez, SRA, GenBank, RefSeq)** | Genomics, sequences, assemblies, taxonomy | REST (E-utils)          | **0**   | ✔️      | Canonical US source                                                        |
| **ENA**                                 | Nucleotide archive, raw reads             | FTP + REST              | **0**   | ✔️      | Mirrors much SRA content                                                   |
| **GENCODE (GTF, FASTA)**                | Gene models, transcripts, proteins        | FTP                     | **0**   | ✔️      | Authoritative gene annotation                                              |
| **ENSEMBL**                             | Annotation, variation, cross-links        | REST                    | **1**   | ✔️      | High-value but API uptime varies                                           |
| **gnomAD**                              | Population allele frequencies             | REST                    | **1**   | ✔️      | Essential variant frequency resource                                       |
| **UCSC Genome Browser**                 | Tracks, chain files, GC content           | HTTP                    | **0**   | ✔️      | Canonical track repository                                                 |
| **GDC (TCGA)**                          | Cancer genomics, WSI metadata             | REST                    | **1**   | ✔️      | Main TCGA access point                                                     |
| **CPTAC**                               | Proteogenomics + WSI                      | HTTPS / portal          | **1**   | ✔️      | Open-access cohorts only; includes WSI metadata + slide reachability tests |
| **TCIA**                                | Radiology + limited pathology             | REST                    | **2**   | ❌       | Canonical radiology collection                                             |
| **Camelyon16/17**                       | Breast cancer WSI                         | HTTPS                   | **2**   | ➖       | Benchmark WSI dataset                                                      |
| **GTEx Histology**                      | Tissue histology images                   | Cloud bucket            | **1–2** | ✔️       | FOV-based; not WSI-native                                                  |
| **UniProt**                             | Protein sequences + metadata              | REST                    | **0**   | ✔️      | Canonical protein namespace                                                |
| **RCSB PDB**                            | Protein structures (3D)                   | GraphQL + REST          | **0**   | ✔️      | Canonical structure repo                                                   |
| **AlphaFold DB**                        | Predicted structures                      | REST                    | **1**   | ❌       | Planned                                                                    |
| **InterPro**                            | Domain / family integration               | REST                    | **1–2** | ❌       | Secondary but important                                                    |
| **Pfam**                                | Protein motifs                            | FTP/REST                | **1**   | ❌       | Planned                                                                    |
| **GO (Gene Ontology)**                  | Biological process ontology               | OBO                     | **0**   | ✔️      | Canonical ontology                                                         |
| **HPO**                                 | Clinical phenotype ontology               | OBO                     | **0**   | ✔️      | Canonical clinical phenotype ontology                                      |
| **MeSH**                                | Medical subject headings                  | REST                    | **0**   | ✔️      | Canonical clinical vocabulary                                              |
| **UMLS**                                | Unified medical language system           | Restricted              | **1**   | ➖       | Licensing-restricted                                                       |
| **SNOMED CT**                           | Clinical ontology                         | Restricted              | **1**   | ➖       | Not open-access                                                            |
| **ClinVar**                             | Variant pathogenicity                     | REST                    | **0**   | ✔️      | Canonical clinical variant database                                        |
| **dbSNP**                               | SNP IDs                                   | REST                    | **0**   | ✔️      | Canonical variant namespace                                                |
| **dbVar**                               | Structural variants                       | REST                    | **0**   | ✔️      | Canonical SV repository                                                    |
| **COSMIC**                              | Somatic mutation catalog                  | Restricted              | **1–2** | ➖       | License required                                                           |
| **BioSamples (EBI)**                    | Sample-level metadata                     | REST                    | **1**   | ❌       | Planned                                                                    |
| **BioStudies / BioProjects**            | Study-level metadata                      | REST                    | **1–2** | ❌       | Planned                                                                    |
| **KEGG**                                | Biological pathways                       | Paid access             | **2–3** | ➖       | Restricted                                                                 |
| **Reactome**                            | Open-access pathways                      | REST                    | **1**   | ❌       | Planned                                                                    |
| **EMPIAR**                              | Electron microscopy (raw EM)              | REST-like JSON + files  | **1**   | ❌       | Planned — primary raw EM archive                                           |
| **EMDB**                                | EM density maps (3D)                      | REST (JSON metadata)    | **2**   | ❌       | Planned — structural EM metadata                                           |
| **Image Data Resource (IDR)**           | Bioimaging (incl. EM)                     | OMERO API / JSON        | **1–2** | ❌       | Planned — microscopy datasets include EM                                   |
| **MitoEM**                              | Volume EM (mitochondria)                  | Download                | **2**   | ➖       | non-API — segmentation benchmark                                           |
| **SNEMI3D**                             | Connectomics EM                           | Download                | **2**   | ➖       | non-API — classic segmentation dataset                                     |
| **CREMI**                               | Connectomics EM                           | Download                | **2**   | ➖       | non-API — EM segmentation benchmark                                        |
| **Kasthuri EM / MICrONS**               | Volume EM (neurons, synapses)             | Python API + download   | **2**   | ❌       | Planned — massive connectomics datasets                                   |
| **EPFL CVLab EM**                       | 2D EM segmentation                        | Download                | **3**   | ➖       | non-API — small classic EM datasets                                       |
| **Allen Cell Structure EM**             | Volume EM (cell ultrastructure)           | Limited API             | **2–3** | ❌       | Planned — high-quality EM volumes                                         |
| **Cell Image Library (CIL)**            | Various EM collections                    | REST metadata           | **2**   | ❌       | Planned — heterogeneous EM microscopy                                     |
| **Imaging Data Commons (IDC)**          | Cancer radiology imaging collections      | DICOMweb + BigQuery     | **2**   | ❌       | Public radiology source; DICOMweb-native access; complements TCIA          |
| **Open-i (NLM)**                        | Biomedical images incl. radiology         | REST search             | **2–3** | ❌       | Useful for indexed radiology images + figure extraction                    |
| **MedPix (NLM)**                        | Radiology teaching cases                  | Web search              | **3**   | ❌       | Semi-structured image metadata; no formal JSON API                         |
| **NIH ChestX-ray14**                    | Chest radiographs                         | Bulk download           | **2**   | ➖       | Benchmark CXR dataset; metadata via CSV                                    |
| **CheXpert**                            | Chest radiographs + labels                | Bulk download           | **2**   | ➖       | High-quality labels; no dynamic API                                        |
| **MIMIC-CXR**                           | Chest radiographs + reports               | Bulk download           | **2**   | ➖       | Requires PhysioNet credentialing; includes reports                         |
| **PadChest**                            | Chest radiographs + NLP labels            | Bulk download           | **2**   | ➖       | Large CXR dataset with Spanish reports                                     |
| **DeepLesion**                          | CT lesion annotations                     | Bulk download           | **2**   | ➖       | NIH CT dataset widely used in radiomics                                    |
| **MIDRC / RICORD**                      | COVID-19 radiology datasets               | Bulk download           | **2**   | ➖       | RSNA/MIDRC collections; mostly via TCIA                                    |
| **BIMCV COVID-19+**                     | Chest X-ray/CT                            | Bulk download           | **3**   | ➖       | Spanish biomedical imaging consortium datasets                             |
| **DrugBank**                            | Drugs, mechanisms, interactions          | REST (restricted)       | **2–3** | ➖       | Paid/commercial license; cannot redistribute                               |
| **ChEMBL**                              | Bioactive molecules, assays              | REST                    | **1**   | ❌       | Major small-molecule/protein bioactivity DB                                 |
| **PubChem**                             | Small molecules, structures, assays      | REST + FTP              | **0–1** | ❌       | Canonical open chemical database                                            |
| **DrugCentral**                         | Drug labels, indications, MoA            | REST                    | **1**   | ❌       | Open-access, excellent structured drug data                                 |
| **BindingDB**                           | Protein–ligand binding affinities         | HTTP/FTP                | **1–2** | ❌       | Large curated affinity dataset                                               |
| **PharmGKB**                            | Pharmacogenomics                         | Restricted              | **2**   | ➖       | Requires registration; licensing limits                                      |
| **ClinicalTrials.gov**                  | Trials, drugs, conditions                 | REST                    | **1**   | ❌       | Canonical source for studies/interventions                                   |
| **Expression Atlas (EBI)**              | Bulk & differential expression            | REST                    | **1**   | ❌       | Gene/condition datasets; links to GEO/SCEA                                   |
| **Single Cell Expression Atlas (SCEA)** | scRNA-seq expression                     | REST + bulk matrices    | **1–2** | ❌       | API only for metadata; expression matrices are downloads                     |
| **GEO (Gene Expression Omnibus)**       | Transcriptomics, arrays, RNA-seq          | N/a                     | **2**   | ➖       | FTP bulk; no usable REST API; not in cyto-vendor-examples                   |
| **Human Cell Atlas (HCA)**              | Single-cell atlases                       | REST (Azul/Matrix)      | **1**   | ❌       | High-value; JSON metadata + matrix endpoints                                 |
| **CellxGene / CZ Biohub**               | Single-cell datasets & annotations        | REST                    | **1–2** | ❌       | API supports dataset metadata & download URLs                                |
| **Tabula Sapiens**                      | Single-cell atlas                         | N/a                     | **2**   | ➖       | Bulk download only                                                            |
| **PanglaoDB**                           | scRNA-seq cell-type markers               | REST                    | **1–2** | ❌       | Useful for cell-type annotation                                               |
| **PRIDE (EBI)**                         | Proteomics datasets                        | REST + FTP              | **1**   | ❌       | Canonical proteomics archive                                                  |
| **MassIVE (UCSD)**                      | Proteomics + metabolomics                  | N/a + partial API       | **2**   | ➖       | Some JSON endpoints; bulk primary access                                      |
| **MetaboLights (EBI)**                  | Metabolomics datasets                      | REST                    | **1–2** | ❌       | Includes sample metadata + SDRF                                                |
| **PeptideAtlas**                        | Peptide evidence across organisms          | N/a                     | **2**   | ➖       | Bulk datasets only                                                             |
| **JUMP-CP / Cell Painting**             | Morphological profiling                    | N/a                     | **2–3** | ➖       | Large S3 bulk imaging; no JSON API                                             |
| **BBBC (Broad Bioimage Benchmark)**     | Microscopy benchmarks                      | N/a                     | **2–3** | ➖       | Bulk TIFF/PNG datasets; no structured API                                      |
| **DeepCell / TissueNet**                | Cell segmentation datasets                 | N/a                     | **2–3** | ➖       | Bulk access; no standardized REST                                              |
| **ZINC15**                               | Drug-like compound library                 | REST                    | **1**   | ❌       | Ligand search and structure metadata                                           |
| **MMDB (NCBI structures)**              | 3D structures                              | REST                    | **1**   | ❌       | Complementary to PDB                                                           |
| **SwissModel Repository**               | Homology models                           | REST                    | **2**   | ❌       | Structure predictions + annotations                                             |
| **BioLip**                              | Ligand–protein interactions               | N/a                     | **2–3** | ➖       | Bulk database of binding residues                                               |
| **LOINC**                               | Lab & clinical measurement codes          | Restricted              | **2**   | ➖       | License-required; clinically essential                                         |
| **ICD-10 / ICD-O**                      | Diagnostic and oncologic codes            | Restricted              | **2–3** | ➖       | WHO licensing; no open REST                                                    |
| **OMOP CDM Vocabularies**               | Clinical vocabularies                      | N/a (download)          | **2**   | ➖       | Semi-restricted; no API                                                         |
| **CDC Wonder**                          | Epidemiology statistics                    | REST                    | **2**   | ❌       | Population-level datasets                                                        |
| **NHANES**                              | Health & nutrition surveys                 | N/a                     | **3**   | ➖       | Bulk tables; no API                                                             |
| **TargetScan**                          | miRNA target predictions                   | N/a                     | **2**   | ➖       | Bulk downloads only; no REST                                                    |
| **miRBase**                             | miRNA sequences & annotations              | FTP/REST (limited)      | **1–2** | ❌       | Canonical miRNA DB; metadata exposed via simple endpoints                      |

---

# **Test Organization**

All vendor examples live in `src/` as `*_test.rs` files.

Organized by domain:

```
gdc_test.rs
gencode_test.rs
ucsc_test.rs
rcsb_test.rs
ncbi_tests.rs
ensembl_tests.rs
ena_test.rs
uniprot_test.rs
obo_test.rs
gnomad_test.rs
…
```

Each file:

* Demonstrates API usage
* Performs strict validation
* Provides copy-pasteable patterns for Cyto vendor modules
* Implements fallback strategies, headers, and better error reporting

All tests run with:

```
cargo test -- --nocapture
```

---

# **Roadmap**

Future expansions will:

* Expand coverage into radiology APIs

---

# **Contributing**

All additions must:

* Include a stable upstream identifier
* Include both a success case and at least one failure mode
* Be resilient to partial outages
* Avoid large downloads (use HEAD/range requests when possible)

Pull requests welcome.

---

# **License**

Apache 2.0 + MIT dual-license
