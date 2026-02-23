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
* ➖ = Not applicable / restricted / under evaluation

---

# **📊 Comprehensive Vendor Table**

Below is the full vendor matrix with **Tier** and **Tested?** status. Vendors that lack an API but can be downloaded in full are not included here (e.g. TargetScan, miRBase)

| Vendor                                  | Domain                                    | API / Access   | Tier    | Tested? | Notes                                                                      |
| --------------------------------------- | ----------------------------------------- | -------------- | ------- | ------- | -------------------------------------------------------------------------- |
| **NCBI (Entrez, SRA, GenBank, RefSeq)** | Genomics, sequences, assemblies, taxonomy | REST (E-utils) | **0**   | ✔️      | Canonical US source                                                        |
| **ENA**                                 | Nucleotide archive, raw reads             | FTP + REST     | **0**   | ✔️      | Mirrors much SRA content                                                   |
| **GENCODE (GTF, FASTA)**                | Gene models, transcripts, proteins        | FTP            | **0**   | ✔️      | Authoritative gene annotation                                              |
| **ENSEMBL**                             | Annotation, variation, cross-links        | REST           | **1**   | ✔️      | High-value but API uptime varies                                           |
| **gnomAD**                              | Population allele frequencies             | REST           | **1**   | ✔️      | Essential variant frequency resource                                       |
| **UCSC Genome Browser**                 | Tracks, chain files, GC content           | HTTP           | **0**   | ✔️      | Canonical track repository                                                 |
| **GDC (TCGA)**                          | Cancer genomics, WSI metadata             | REST           | **1**   | ✔️      | Main TCGA access point                                                     |
| **CPTAC**                               | Proteogenomics + WSI                      | HTTPS / portal | **1**   | ✔️      | Open-access cohorts only; includes WSI metadata + slide reachability tests |
| **TCIA**                                | Radiology + limited pathology             | REST           | **2**   | ❌       | Secondary support                                                          |
| **Camelyon16/17**                       | Breast cancer WSI                         | HTTPS          | **2**   | ❌       | Benchmark WSI dataset                                                      |
| **GTEx Histology**                      | Tissue histology images                   | Cloud bucket   | **1–2** | ✔️       | FOV-based; not WSI-native                                                 |
| **UniProt**                             | Protein sequences + metadata              | REST           | **0**   | ✔️      | Canonical protein namespace                                                |
| **RCSB PDB**                            | Protein structures (3D)                   | GraphQL + REST | **0**   | ✔️      | Canonical structure repo                                                   |
| **AlphaFold DB**                        | Predicted structures                      | REST           | **1**   | ❌       | Planned                                                                    |
| **InterPro**                            | Domain / family integration               | REST           | **1–2** | ❌       | Secondary but important                                                    |
| **Pfam**                                | Protein motifs                            | FTP/REST       | **1**   | ❌       | Planned                                                                    |
| **GO (Gene Ontology)**                  | Biological process ontology               | OBO            | **0**   | ✔️      | Canonical ontology                                                         |
| **HPO**                                 | Clinical phenotype ontology               | OBO            | **0**   | ✔️      | Canonical clinical phenotype ontology                                      |
| **MeSH**                                | Medical subject headings                  | REST           | **0**   | ✔️      | Canonical clinical vocabulary                                              |
| **UMLS**                                | Unified medical language system           | Restricted     | **1**   | ➖       | Licensing-restricted                                                       |
| **SNOMED CT**                           | Clinical ontology                         | Restricted     | **1**   | ➖       | Not open-access                                                            |
| **ClinVar**                             | Variant pathogenicity                     | REST           | **0**   | ✔️      | Canonical clinical variant database                                        |
| **dbSNP**                               | SNP IDs                                   | REST           | **0**   | ✔️      | Canonical variant namespace                                                |
| **dbVar**                               | Structural variants                       | REST           | **0**   | ✔️      | Canonical SV repository                                                    |
| **COSMIC**                              | Somatic mutation catalog                  | Restricted     | **1–2** | ➖       | License required                                                           |
| **BioSamples (EBI)**                    | Sample-level metadata                     | REST           | **1**   | ❌       | Planned                                                                    |
| **BioStudies / BioProjects**            | Study-level metadata                      | REST           | **1–2** | ❌       | Planned                                                                    |
| **KEGG**                                | Biological pathways                       | Paid access    | **2–3** | ➖       | Restricted                                                                 |
| **Reactome**                            | Open-access pathways                      | REST           | **1**   | ❌       | Planned                                                                    |

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
