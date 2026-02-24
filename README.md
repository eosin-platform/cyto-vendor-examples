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

Below is the full vendor matrix with **Tier** and **Tested?** status.

| Vendor                                 | Domain                                  | API / Access              | Tier    | Tested? | Notes                                                                      |
| -------------------------------------- | ---------------------------------------- | -------------------------- | ------- | ------- | -------------------------------------------------------------------------- |
| **NCBI (Entrez, SRA, GenBank, RefSeq)** | Genomics, sequences, assemblies, taxonomy | REST (E-utils)            | **0**   | ✔️      | Canonical US source                                                        |
| **ENA**                                 | Nucleotide archive, raw reads             | FTP + REST                | **0**   | ✔️      | Mirrors much SRA content                                                   |
| **GENCODE (GTF, FASTA)**                | Gene models, transcripts, proteins        | FTP                       | **0**   | ✔️      | Authoritative gene annotation                                              |
| **UCSC Genome Browser**                 | Tracks, chain files, GC content           | HTTP                      | **0**   | ✔️      | Canonical track repository                                                 |
| **UniProt**                             | Protein sequences + metadata              | REST                      | **0**   | ✔️      | Canonical protein namespace                                                |
| **RCSB PDB**                            | Protein structures (3D)                   | GraphQL + REST            | **0**   | ✔️      | Canonical structure repo                                                   |
| **GO (Gene Ontology)**                  | Biological process ontology               | OBO                       | **0**   | ✔️      | Canonical ontology                                                         |
| **HPO**                                 | Clinical phenotype ontology               | OBO                       | **0**   | ✔️      | Canonical clinical phenotype ontology                                      |
| **MeSH**                                | Medical subject headings                  | REST                      | **0**   | ✔️      | Canonical clinical vocabulary                                              |
| **ClinVar**                             | Variant pathogenicity                     | REST                      | **0**   | ✔️      | Canonical clinical variant database                                        |
| **dbSNP**                               | SNP IDs                                   | REST                      | **0**   | ✔️      | Canonical SNP identifier namespace                                         |
| **dbVar**                               | Structural variants                       | REST                      | **0**   | ✔️      | Canonical structural variant repository                                    |
| **ENSEMBL**                             | Annotation, variation, cross-links        | REST                      | **1**   | ✔️      | High-value but API uptime varies                                           |
| **gnomAD**                              | Population allele frequencies             | REST                      | **1**   | ✔️      | Essential variant frequency resource                                       |
| **GDC (TCGA)**                          | Cancer genomics, WSI metadata             | REST                      | **1**   | ✔️      | Main TCGA access point                                                     |
| **CPTAC**                               | Proteogenomics + WSI                      | HTTPS / portal            | **1**   | ✔️      | Open-access cohorts only; includes WSI metadata                            |
| **GTEx Histology**                      | Tissue histology images                   | Cloud bucket              | **1–2** | ✔️      | FOV-based histology; not WSI-native                                        |
| **AlphaFold DB**                        | Predicted protein structures              | REST                      | **1**   | ❌       | Planned                                                                    |
| **InterPro**                            | Protein families & integration             | REST                      | **1–2** | ❌       | Annotates protein domains across multiple databases                        |
| **Pfam**                                | Protein motifs & domains                  | FTP / REST                | **1**   | ❌       | Protein family models                                                       |
| **Reactome**                            | Biological pathways                        | REST                      | **1**   | ❌       | Open-access pathway knowledgebase                                          |
| **Expression Atlas (EBI)**              | Bulk & differential expression             | REST                      | **1**   | ❌       | Microarray & RNA-seq expression across conditions                          |
| **Single Cell Expression Atlas (SCEA)** | Single-cell expression data                | REST + N/a                | **1–2** | ❌       | API for metadata only; matrices are bulk                                   |
| **Human Cell Atlas (HCA)**              | Single-cell atlases                        | REST (Azul/Matrix)        | **1**   | ❌       | JSON metadata + matrix endpoints                                           |
| **CellxGene / CZ Biohub**               | scRNA-seq datasets & annotations           | REST                      | **1–2** | ❌       | Metadata API + dataset downloads                                           |
| **PanglaoDB**                           | scRNA-seq cell-type markers                | REST                      | **1–2** | ❌       | Useful for cell-type assignment                                             |
| **PRIDE (EBI)**                         | Proteomics datasets                        | REST + FTP                | **1**   | ❌       | Canonical proteomics submissions                                           |
| **MetaboLights (EBI)**                  | Metabolomics datasets                      | REST                      | **1–2** | ❌       | SDRF-based metadata                                                        |
| **MGnify (EBI Metagenomics)**           | Metagenomics, microbiomes                  | REST                      | **1**   | ❌       | Canonical pipeline outputs for microbiome data                             |
| **BV-BRC (PATRIC / BRC)**               | Bacteria & viruses, AMR, virulence         | REST                      | **1**   | ❌       | Major pathogen informatics resource                                        |
| **NCBI Virus**                          | Viral genomes & metadata                   | REST                      | **1**   | ❌       | Public viral sequences; structured metadata                                |
| **CIViC**                               | Clinical variant interpretations            | REST                      | **1**   | ❌       | Curated oncology variant evidence                                          |
| **ZINC15**                               | Small-molecule ligand library               | REST                      | **1**   | ❌       | Searchable compound metadata                                               |
| **MMDB (NCBI)**                         | 3D structures                               | REST                      | **1**   | ❌       | Complementary to PDB                                                       |
| **SABIO-RK**                            | Enzyme reaction kinetics                    | REST                      | **1–2** | ❌       | Structured biochemical kinetics                                             |
| **LIPID MAPS**                          | Lipids & pathways                           | REST                      | **1–2** | ❌       | Lipid classifications & reactions                                          |
| **BioGRID**                             | Protein interactions & genetics             | REST                      | **1**   | ❌       | Canonical PPI/interaction dataset                                          |
| **STRING DB**                           | Protein interaction networks                | REST                      | **1**   | ❌       | Network-based biological associations                                      |
| **IntAct (EBI)**                        | Molecular interaction evidence              | REST                      | **1–2** | ❌       | Curated interaction repository                                             |
| **ComplexPortal**                       | Protein complexes                            | REST                      | **1–2** | ❌       | Structured protein complex definitions                                     |
| **ChEBI**                               | Chemical ontology & structures              | REST                      | **1**   | ❌       | Chemical entity ontology                                                   |
| **TCIA**                                | Radiology collections                       | REST                      | **2**   | ❌       | Canonical radiology repository                                             |
| **Imaging Data Commons (IDC)**          | Cancer radiology collections                 | DICOMweb + BigQuery       | **2**   | ❌       | Cloud-hosted DICOM; complements TCIA                                       |
| **Open-i (NLM)**                        | Biomedical figures incl. radiology           | REST search               | **2–3** | ❌       | Image search engine                                                        |
| **ClinicalTrials.gov**                  | Trials, drugs, conditions                    | REST                      | **1–2** | ❌       | Canonical clinical trials registry                                         |
| **DrugCentral**                         | Drug labels, indications, MoA                | REST                      | **1**   | ❌       | Open pharmacology resource                                                 |
| **ChEMBL**                              | Bioactive molecules, assays                  | REST                      | **1**   | ❌       | Major chemical biology resource                                            |
| **PubChem**                             | Small molecules, assays, structures           | REST + FTP                | **0–1** | ❌       | Canonical chemical database                                                |
| **BioSamples (EBI)**                    | Sample metadata                               | REST                      | **1**   | ❌       | Underpins many EBI datasets                                                |
| **BioStudies / BioProjects**            | Study metadata                                | REST                      | **1–2** | ❌       | Study-level organization                                                   |
| **OncoKB**                              | Precision oncology knowledgebase              | REST (restricted)         | **1–2** | ➖       | API key required                                                           |
| **LOINC**                               | Lab/clinical measurement codes                | Restricted                | **2**   | ➖       | Controlled terminology                                                     |
| **ICD-10 / ICD-O**                      | Diagnostic and oncologic codes                | Restricted                | **2–3** | ➖       | WHO licensing                                                              |
| **COSMIC**                              | Somatic mutation catalog                      | Restricted                | **1–2** | ➖       | Commercial license                                                         |
| **DrugBank**                            | Drugs, interactions, mechanisms               | REST (restricted)         | **2–3** | ➖       | Paid API; cannot redistribute                                              |
| **BioCyc / MetaCyc**                    | Pathways & metabolism                         | Restricted                | **2–3** | ➖       | Paid licensing; limited API                                                |
| **BRENDA**                              | Enzyme kinetics                                | Restricted                | **2–3** | ➖       | Licensing required                                                         |
| **GISAID**                              | Viral sequences (SARS-CoV-2, influenza)       | Restricted                | **2–3** | ➖       | No public API; controlled-access                                           |
| **IMG/M (JGI)**                         | Microbial genomes & metagenomes               | Restricted                | **2–3** | ➖       | Login required                                                             |
| **NHANES**                              | Health & nutrition surveys                     | N/a                       | **3**   | ➖       | Bulk tables only; no API                                                   |
| **CDC Wonder**                          | Epidemiology statistics                        | REST                      | **2**   | ❌       | Global & US health stats                                                   |
| **Our World in Data (OWID)**            | Global health metrics                          | REST                      | **2**   | ❌       | Epidemiology & mortality data                                              |
| **WHO GHO API**                         | World health statistics                        | REST                      | **2**   | ❌       | Official WHO global health API                                             |
| **UNData API**                          | Population & health statistics                 | REST                      | **3**   | ❌       | Broad global datasets                                                      |
| **Camelyon16/17**                       | Breast cancer WSI                             | N/a                       | **2**   | ➖       | Bulk-only WSI dataset                                                      |
| **NIH ChestX-ray14**                    | Chest radiographs                              | N/a                       | **2**   | ➖       | Bulk dataset; no API                                                       |
| **CheXpert**                            | Chest radiographs + labels                     | N/a                       | **2**   | ➖       | Bulk-only; no API                                                          |
| **MIMIC-CXR**                           | Chest radiographs + reports                    | N/a                       | **2**   | ➖       | Bulk PhysioNet; no public API                                              |
| **PadChest**                            | Chest radiographs + NLP labels                 | N/a                       | **2**   | ➖       | Bulk-only dataset                                                          |
| **DeepLesion**                          | CT lesion dataset                               | N/a                       | **2**   | ➖       | Bulk-only; no API                                                           |
| **MIDRC / RICORD**                      | COVID-19 radiology                              | N/a                       | **2**   | ➖       | Bulk-only from TCIA                                                        |
| **BIMCV COVID-19+**                     | Chest X-ray/CT                                   | N/a                       | **3**   | ➖       | Bulk-only                                                                   |
| **EMPIAR**                              | Raw EM volumes                                  | REST-like + N/a           | **1**   | ❌       | Image metadata API; volumes bulk-only                                      |
| **EMDB**                                | EM density maps                                  | REST                      | **2**   | ❌       | Structured metadata                                                        |
| **Image Data Resource (IDR)**           | Microscopy & EM                                   | REST (OMERO)              | **1–2** | ❌       | Metadata API; images bulk                                                  |
| **BioImage Archive (EBI)**              | Microscopy datasets                               | REST metadata             | **1–2** | ❌       | General microscopy repository                                              |
| **Kasthuri EM / MICrONS**               | Connectomics EM                                   | N/a                       | **2**   | ➖       | Bulk large-scale volumes                                                   |
| **MitoEM**                              | Mitochondrial EM segmentation                     | N/a                       | **2**   | ➖       | Benchmark dataset; no API                                                  |
| **SNEMI3D**                             | Connectomics EM                                   | N/a                       | **2**   | ➖       | Classic EM segmentation benchmark                                          |
| **CREMI**                               | Connectomics EM                                   | N/a                       | **2**   | ➖       | Bulk segmentation benchmark                                                |
| **EPFL CVLab EM**                       | 2D EM segmentation                                | N/a                       | **3**   | ➖       | Small EM dataset                                                           |
| **Allen Cell Explorer**                 | Live-cell imaging                                 | N/a                       | **2–3** | ➖       | High-quality imaging; no API                                               |
| **CEM500K**                             | Connectomics EM                                    | N/a                       | **3**   | ➖       | Bulk-only large dataset                                                     |
| **CARD (AMR)**                          | Antibiotic resistance genes                      | N/a                       | **2–3** | ➖       | Bulk-only AMR gene database                                                |
| **VFDB**                                | Bacterial virulence factors                       | N/a                       | **2–3** | ➖       | Bulk-only; no API                                                          |
| **SILVA**                               | rRNA reference sequences                           | N/a                       | **2**   | ➖       | Widely used microbiome taxonomy                                            |
| **Greengenes2**                         | Microbial taxonomy                                  | N/a                       | **2–3** | ➖       | Updated Greengenes; no API                                                 |
| **RDP Classifier**                      | 16S reference classifier                            | N/a                       | **3**   | ➖       | Training datasets only                                                     |
| **TargetScan**                          | miRNA target predictions                            | N/a                       | **2**   | ➖       | Bulk-only; no REST API                                                     |
| **miRBase**                             | microRNA sequences & annotations                    | FTP / REST (limited)       | **1–2** | ❌       | Limited API endpoints                                                      |
| **HMDB (Human Metabolome DB)**          | Metabolomics                                         | N/a + limited API          | **2**   | ➖       | Partial API; primary access bulk                                           |
| **MassIVE (UCSD)**                      | Proteomics & metabolomics                            | N/a + partial API          | **2**   | ➖       | Some JSON endpoints                                                        |
| **PeptideAtlas**                        | Peptide evidence                                     | N/a                       | **2**   | ➖       | Bulk datasets                                                              |
| **Cell Ontology (CL)**                  | Cell-type ontology                                  | OBO                       | **0–1** | ❌       | Core cell ontology                                                          |
| **Uberon**                              | Anatomy ontology                                     | OBO                       | **0–1** | ❌       | Multi-species anatomy                                                       |
| **Sequence Ontology (SO)**              | Sequence feature ontology                            | OBO                       | **0–1** | ❌       | Used heavily in genome annotation                                           |
| **Disease Ontology (DO)**               | Disease classification                               | OBO                       | **0–1** | ❌       | Open disease ontology                                                       |
| **HGMD (Human Gene Mutation Database)** | Clinical variants                          | Restricted              | **2–3** | ➖       | Commercial clinical mutation database; no public API                      |
| **VarSome**                             | Variant annotation & ACMG classification   | Restricted (REST)       | **2**   | ➖       | Paid API; integrated knowledge graph                                      |
| **DECIPHER**                            | Clinical structural variation              | REST                    | **1–2** | ❌       | Patient phenotypes + CNVs; partial API                                   |
| **LOVD**                                | Community gene/variant databases           | REST                    | **1–2** | ❌       | Variant submissions for specific genes                                   |
| **MGI (Mouse Genome Informatics)**      | Model organism — mouse                     | REST                    | **1**   | ❌       | Mouse genetics, alleles, phenotypes                                      |
| **RGD (Rat Genome Database)**           | Model organism — rat                       | REST                    | **1**   | ❌       | Rat genomics, disease models, pathways                                   |
| **FlyBase**                             | Model organism — Drosophila                | REST                    | **1**   | ❌       | Fly gene models, alleles, interactions                                   |
| **WormBase / WBPS**                     | Model organism — C. elegans                | REST                    | **1**   | ❌       | Genes, phenotypes, pathways                                              |
| **ZFIN**                                | Model organism — zebrafish                 | REST                    | **1**   | ❌       | Developmental phenotypes, expression                                     |
| **SGD (Yeast Genome Database)**         | Model organism — yeast                     | REST                    | **1**   | ❌       | Saccharomyces cerevisiae reference DB                                    |
| **Xenbase**                             | Model organism — Xenopus                   | REST                    | **1–2** | ❌       | Frog developmental biology                                               |
| **Phytozome / Araport / TAIR**          | Plant genomics                             | Restricted              | **3**   | ➖       | Arabidopsis & plant resources; licensing required                        |
| **Gramene**                             | Plant comparative genomics                 | REST                    | **2**   | ❌       | Phylogenomic and annotation resource                                     |
| **Plant Reactome**                      | Plant pathways                              | REST                    | **2**   | ❌       | Plant pathway knowledgebase                                              |
| **OMA (Orthologous Matrix)**            | Orthology                                   | REST                    | **1–2** | ❌       | High-quality ortholog predictions                                        |
| **OrthoDB**                             | Ortholog clusters                           | REST                    | **1–2** | ❌       | Hierarchical orthology database                                          |
| **Ensembl Compara**                     | Orthology / phylogeny                       | REST                    | **1**   | ❌       | Part of Ensembl, but separate API endpoints                              |
| **GWAS Catalog**                        | Genome-wide association studies             | REST                    | **1**   | ❌       | Curated trait–variant associations                                       |
| **eQTL Catalogue**                      | eQTL & QTL studies                          | REST                    | **1**   | ❌       | Uniform reprocessing of QTL datasets                                     |
| **UK Biobank**                          | Genomics & clinical cohort                  | Restricted              | **3**   | ➖       | Controlled-access dataset                                                |
| **BioBank Japan**                       | Clinical biobank                             | Restricted              | **3**   | ➖       | Large Japanese cohort; controlled access                                 |
| **IUPHAR Guide to Pharmacology**        | Drug targets, ligands, GPCRs                | REST                    | **1–2** | ❌       | Pharmacology reference curated by IUPHAR                                 |
| **ChemSpider**                          | Chemical structures                          | REST (key required)      | **2**   | ➖       | API requires registration; broad chemical metadata                       |
| **Addgene (already listed)**            | –                                            | –                        | –       | –        | (skip — already included)                                                |
| **PHASTER**                             | Prophage & phage element annotation         | REST                    | **1–2** | ❌       | Bacteriophage detection in microbial genomes                             |
| **EnteroBase**                          | Enteric pathogen genomics                   | REST                    | **1–2** | ❌       | Salmonella, E. coli, Campylobacter, more                                 |
| **IMG/VR (Virus/Phage)**                | Viral & phage genomes                       | Restricted              | **2–3** | ➖       | Part of JGI IMG ecosystem; login needed                                  |
| **CRyPTIC TB AMR Database**             | Tuberculosis resistance profiles            | N/a                     | **3**   | ➖       | Bulk-only; no API                                                        |
| **MetaPhlAn database**                  | Microbiome taxonomic markers                | N/a                     | **3**   | ➖       | Bulk reference database only                                             |
| **Kraken2 / Bracken DBs**               | Microbiome classification indices            | N/a                     | **3**   | ➖       | Bulk-only k-mer taxonomic databases                                      |
| **GTDB (Genome Taxonomy Database)**     | Microbial taxonomy                          | N/a                     | **2–3** | ➖       | Widely used in metagenomics; bulk-only                                   |
| **Earth Microbiome Project**            | Microbiome survey                            | N/a                     | **3**   | ➖       | Bulk metadata + sequence archives                                        |
| **BMRB (Biological Magnetic Resonance DB)** | NMR spectroscopy                        | REST                    | **2**   | ❌       | Protein & metabolite NMR data                                             |
| **OpenFold training data**              | Protein structure ML datasets               | N/a                     | **3**   | ➖       | Bulk ML training sets                                                     |
| **OpenStructure datasets**              | Structural biology ML sets                  | N/a                     | **3**   | ➖       | Bulk ML training assets                                                   |
| **EuropePMC**                           | Literature metadata                         | REST                    | **1–2** | ❌       | Large open biomedical literature store                                    |
| **Semantic Scholar API**                | Scholarly metadata & citations              | REST                    | **2**   | ❌       | Good for NLP & citation graph extraction                                 |
| **CORD-19**                             | COVID-19 research corpus                     | N/a                     | **3**   | ➖       | Bulk paper corpus                                                         |
| **NCBI BioSample**                      | Biological sample metadata                   | REST                    | **1**   | ❌       | Parallel to EBI BioSamples                                                |
| **iGEM Registry of Standard Parts**     | Synthetic biology parts                     | REST (MediaWiki API)    | **2**   | ❌       | DNA parts, promoters, plasmids                                            |
| **JBEI ICE Repository**                 | SynBio plasmids & parts                     | REST                    | **2**   | ❌       | Open-source ICE platform                                                  |
| **SynBioHub**                           | Synthetic biology parts                     | REST                    | **2**   | ❌       | SBOL-formatted biological components                                      |
| **dbNSFP**                              | Variant scoring annotations                  | N/a                     | **2**   | ➖       | Bulk functional prediction database                                       |
| **CADD**                                | Variant deleteriousness scores               | N/a                     | **2–3** | ➖       | Bulk prediction files only                                                |
| **PolyPhen / SIFT**                     | Protein-level variant effect prediction      | N/a                     | **3**   | ➖       | Web servers only; no API                                                  |
| **REVEL / M-CAP / MutPred**             | Variant effect prediction                    | N/a                     | **3**   | ➖       | Bulk downloads only                                                       |
| **MetaCyc / BioCyc (already included)** | –                                            | –                        | –       | –        | (skip)                                                                  |
| **GTEx Expression (portal)**            | Expression quantifications                   | REST                    | **1**   | ❌       | Distinct from histology; expression matrices via API                     |
| **Earth Human Microbiome Project**      | Human microbiome                             | N/a                     | **3**   | ➖       | Bulk-only metadata                                                       |
| **FHIR Terminology Service (HL7)**      | Clinical vocab expansions                    | REST                    | **1–2** | ❌       | Provides ValueSet expansion & concept lookup                             |
| **Allen Brain Atlas**                   | CNS gene expression & cell types             | REST                    | **1–2** | ❌       | Spatial transcriptomics + expression                                     |
| **EyePACS Diabetic Retinopathy**        | Retinal imaging                             | N/a                     | **2**   | ➖       | Bulk ophthalmology dataset; no API                                         |
| **APTOS / Kaggle DR**                   | Retinal imaging                             | N/a                     | **2**   | ➖       | Bulk-only DR classification dataset                                        |
| **MESSIDOR**                            | Retinal imaging                             | N/a                     | **2**   | ➖       | Classic DR imaging benchmark; no API                                      |
| **ISIC Archive**                        | Dermatology / skin cancer                   | REST                    | **1–2** | ❌       | Public skin lesion image archive                                           |
| **HAM10000**                            | Dermatology / skin cancer                   | N/a                     | **2**   | ➖       | Bulk lesion classification dataset                                         |
| **PH2 Dataset**                         | Dermatology (dermoscopy)                    | N/a                     | **3**   | ➖       | Small bulk-only dermoscopy dataset                                        |
| **ENCODE**                              | Epigenomics, regulatory genomics            | REST                    | **1**   | ❌       | ATAC-seq, ChIP-seq, DNase-seq, TF binding                                 |
| **Roadmap Epigenomics**                 | Epigenomics                                  | N/a                     | **2**   | ➖       | Bulk-only consortium data                                                  |
| **ReMap**                               | Transcription factor binding                 | REST                    | **1–2** | ❌       | Integrated TF-binding meta-analysis                                        |
| **Cistrome Data Browser**               | ChIP-seq / ATAC-seq regulatory data          | REST                    | **1–2** | ❌       | TF and histone mark binding                                                |
| **GTRD (TF Binding Database)**          | Transcription factor binding sites            | N/a                     | **2–3** | ➖       | Aggregated TF-binding predictions                                          |
| **JASPAR**                              | TF binding motifs                             | REST                    | **1–2** | ❌       | Canonical open TF motif database                                           |
| **10x Genomics Visium Datasets**        | Spatial transcriptomics                      | N/a                     | **2**   | ➖       | Bulk dataset downloads; no public REST API                                 |
| **SpatialDB**                           | Spatial transcriptomics index                 | REST                    | **1–2** | ❌       | Aggregates spatial-omics datasets                                          |
| **STOmics (BGI)**                       | Spatial omics                                 | N/a                     | **2–3** | ➖       | Bulk downloads; no public API                                              |
| **Slide-seq / Slide-seqV2**             | Spatial transcriptomics                      | N/a                     | **3**   | ➖       | Bulk bead-based spatial RNA-seq                                           |
| **STARmap**                             | Spatial transcriptomics                      | N/a                     | **3**   | ➖       | High-resolution spatial RNA imaging                                         |
| **MERFISH datasets**                    | Spatial transcriptomics                      | N/a                     | **3**   | ➖       | Bulk-only spatial imaging data                                              |
| **TriTrypDB**                           | Parasitology genomics (Trypanosomes)         | REST                    | **1–2** | ❌       | Part of EuPathDB                                                           |
| **PlasmoDB**                            | Malaria parasite genomics                    | REST                    | **1–2** | ❌       | Major Plasmodium resource                                                  |
| **VectorBase**                          | Vector insect genomics (mosquitoes)          | REST                    | **1–2** | ❌       | Mosquito, tick, sandfly genomics                                           |
| **FungiDB**                             | Fungal pathogens                             | REST                    | **1–2** | ❌       | Fungal parasite genomics                                                   |
| **EuPathDB (umbrella)**                 | Parasitology databases                        | REST                    | **1–2** | ❌       | Aggregates PlasmoDB, TriTrypDB, ToxoDB, more                               |
| **Metabolomics Workbench**              | Metabolomics                                  | REST                    | **1–2** | ❌       | NIH metabolomics repository                                                |
| **MassBank (Japan)**                    | Mass spectra (metabolomics)                   | REST                    | **2**   | ❌       | High-quality MS spectra reference                                          |
| **MoNA (MassBank of North America)**    | Mass spectra                                   | REST                    | **2**   | ❌       | North American MS spectra archive                                          |
| **ToxCast**                             | Toxicology screens (HTS)                      | REST                    | **1–2** | ❌       | EPA high-throughput toxicity assays                                        |
| **Tox21**                               | Toxicology screens                             | N/a                     | **2**   | ➖       | Bulk HTS toxicology dataset                                                |
| **EPA CompTox Dashboard**               | Chemical toxicity & exposure                  | REST                    | **1–2** | ❌       | Unified EPA toxicology/chemical metadata                                  |
| **CTD (Comparative Toxicogenomics DB)** | Gene–chemical–disease interactions             | REST                    | **1–2** | ❌       | Toxico-genomic relationships                                               |
| **AIRR Data Commons**                   | Immune repertoire sequences (TCR/BCR)         | REST                    | **1–2** | ❌       | Standardized immune repertoire metadata                                   |
| **VDJdb**                               | T-cell receptor sequences                      | REST                    | **1–2** | ❌       | Curated TCR specificity database                                           |
| **ImmPort**                             | Immunology studies                             | REST                    | **1–2** | ❌       | NIH immunology data portal                                                 |
| **IEDB**                                | Immune epitope database                        | REST                    | **1–2** | ❌       | Antibody & T-cell epitope repository                                      |
| **MIMIC-IV (clinical EHR)**             | Clinical EHR, ICU data                         | Restricted              | **2–3** | ➖       | PhysioNet credential required; no open API                                |
| **eICU Collaborative Research DB**      | ICU clinical data                               | Restricted              | **2–3** | ➖       | Clinical EHR-like dataset                                                  |
| **OMOP Example Data Sets**              | Clinical EHR example datasets                   | N/a                     | **3**   | ➖       | Demo EHR tables; no API                                                    |
| **NeuroMorpho.org**                     | Neuronal morphology reconstructions            | REST                    | **1–2** | ❌       | Canonical neuron structure repository                                     |
| **OpenNeuro**                           | Neuroimaging (MRI/MEG/EEG) datasets            | REST + N/a              | **2**   | ❌       | Metadata API; imaging bulk                                                 |
| **Human Connectome Project (HCP)**      | Human brain MRI & connectomics                 | Restricted              | **2–3** | ➖       | Access-controlled large neuroimaging dataset                               |
| **GlyGen**                              | Glycans, glycoproteins, glycan interactions    | REST                    | **1–2** | ❌       | Glycomics metadata & structures                                            |
| **CAZy**                                | Carbohydrate-active enzymes                    | N/a                     | **2–3** | ➖       | No API; curated carbohydrate enzyme families                               |
| **Exposome Explorer**                   | Human chemical exposures                       | N/a                     | **3**   | ➖       | Epidemiological exposome database                                          |
| **EPA Envirofacts**                     | Environmental chemical & pollutant data         | REST                    | **2–3** | ❌       | Environmental exposure APIs                                                |
| **PubMed API (E-Utilities)**            | Literature metadata                             | REST                    | **0–1** | ❌       | Explicit listing for literature search                                     |
| **OBO Foundry Registry**                | Ontology metadata                               | N/a                     | **3**   | ➖       | Registry of OBO ontologies; no API                                         |
| **IHME / GBD (Global Burden of Disease)** | Epidemiology & mortality                       | REST                    | **2–3** | ❌       | Global disease burden datasets                                             |
| **UN WHO Mortality Tables**             | Mortality statistics                            | N/a                     | **3**   | ➖       | Bulk-download-only                                                         |
| **Cellosaurus**                          | Cell-line ontology & metadata             | REST                    | **1–2** | ❌       | Canonical cell-line registry; IDs widely used in research                 |
| **DepMap**                               | Cancer cell-line genetics (CRISPR, RNAi)  | REST + bulk             | **1–2** | ❌       | Gene essentiality, copy-number, expression; CCLE-aligned                  |
| **CCLE (Cancer Cell Line Encyclopedia)** | Cancer cell-line molecular profiles       | N/a + partial API       | **2**   | ➖       | Expression/CNV/mutation data; bulk downloads primarily                     |
| **PomBase**                              | Model organism — fission yeast            | REST                    | **1–2** | ❌       | Schizosaccharomyces pombe gene models, alleles, interactions              |
| **Rhea**                                 | Curated biochemical reactions              | REST                    | **1–2** | ❌       | High-quality enzyme reaction knowledgebase linked to UniProt              |
| **ECOD**                                 | Protein domain classification              | N/a                     | **2–3** | ➖       | Hierarchical structural domain database; bulk-only                         |
| **FooDB**                                | Food constituent metabolomics              | REST                    | **2**   | ❌       | Food-related chemical compounds linked to nutrition & metabolomics        |
| **USDA FoodData Central**                | Nutrition & biochemical food composition   | REST                    | **2**   | ❌       | Authoritative nutrient profiles; useful for diet–metabolomics studies     |
| **ClinGen Dosage Map**                  | Gene dosage sensitivity                   | REST                    | **1–2** | ❌       | Haploinsufficiency & triplosensitivity curation                           |
| **IMGT**                                | Immunogenetics (TCR/BCR)                  | REST                    | **1–2** | ❌       | Canonical antibody & receptor sequence DB                                  |
| **ImmPort**                             | Immunology datasets                        | REST                    | **1–2** | ❌       | Immune profiling & flow/mass cytometry data                                |
| **AIRR Data Commons**                   | Immune repertoire sequencing               | REST                    | **1–2** | ❌       | Adaptive immune receptor repertoires                                        |
| **dbGaP**                               | Controlled-access human genomics           | Restricted              | **3**   | ➖       | Massive human cohort datasets; no public API                               |
| **Genome Nexus**                        | Variant annotation aggregator              | REST                    | **1–2** | ❌       | Harmonizes ClinVar, COSMIC, OncoKB, gnomAD                                  |
| **SnpEff DB**                           | Variant annotation reference               | N/a                     | **2**   | ➖       | Used by SnpEff pipelines; downloaded reference sets                         |
| **VEP Cache**                            | Variant Effect Predictor reference          | N/a                     | **2**   | ➖       | Offline Ensembl VEP annotation caches                                      |
| **gnomAD-SV**                           | Structural variants                        | REST                    | **1–2** | ❌       | Separate SV endpoints from SNVs                                             |
| **PAMDB**                               | Pseudomonas metabolomics                   | N/a                     | **2–3** | ➖       | Bulk downloads; bacteria-specific metabolome                               |
| **MiGA**                                 | Microbial genome classification             | REST                    | **1–2** | ❌       | Taxonomic & phylogenomic classification                                    |
| **ENVO (Environment Ontology)**          | Environmental metadata ontology             | OBO                     | **0–1** | ❌       | Used in microbiome & metagenomics metadata                                 |
| **Veterinary Pathogen DBs (APHA / VeNom)** | Animal pathogens & clinical terms     | N/a / restricted        | **3**   | ➖       | Used in veterinary microbiology & epidemiology                              |
| **PubChem BioAssay (AID)**              | HTS bioassays                               | REST + FTP              | **1**   | ❌       | Massive assay database; essential for chemogenomics                        |
| **ToxCast / Tox21**                      | Toxicology HTS assays                       | REST + bulk             | **1–2** | ❌       | EPA’s high-throughput toxicity screens                                     |
| **Open Targets Platform**               | Drug–gene–disease graph                     | REST                    | **1**   | ❌       | High-quality integrated biomedical knowledge graph                          |
| **FAERS**                               | FDA adverse event reports                   | REST                    | **1–2** | ❌       | OpenFDA pharmacovigilance API                                              |
| **SIDER**                               | Drug side effects                           | N/a                     | **2**   | ➖       | Side-effect profiles; bulk structured data                                  |
| **DrugSideEffectsDB**                   | Drug adverse effect resource                | N/a                     | **2–3** | ➖       | Alternative side-effect knowledgebase                                       |
| **BrainMap / BrainInfo**                | Neuroanatomy ontology                       | REST                    | **1–2** | ❌       | Classic neuroanatomy structured ontology                                   |
| **NeuroMorpho.org**                     | Neuronal morphologies                       | REST                    | **1–2** | ❌       | Digital neuron reconstructions                                              |
| **Allen Brain Observatory**             | Ephys & imaging                             | REST                    | **1–2** | ❌       | Calcium imaging, electrophysiology datasets                                 |
| **OpenNeuro**                           | MRI/EEG neuroimaging                        | N/a                     | **2**   | ➖       | BIDS datasets; bulk-only                                                    |
| **Neurosynth**                          | fMRI meta-analysis                          | REST                    | **1–2** | ❌       | Automated cognitive-neuroscience associations                               |
| **NITRC**                               | Neuroimaging tools & data                   | N/a                     | **2–3** | ➖       | Repository of MRI/EEG datasets                                              |
| **Human Connectome Project (HCP)**      | Connectomics (MRI)                          | Restricted              | **3**   | ➖       | Controlled-access MRI & phenotype data                                      |
| **ABCD Study**                          | Child brain development (MRI, psych)        | Restricted              | **3**   | ➖       | Controlled-access high-value cohort                                          |
| **OpenFDA**                             | Drugs, devices, adverse events              | REST                    | **1–2** | ❌       | FDA regulatory + safety data                                                |
| **CMS Medicare Data**                   | Utilization & reimbursement                 | N/a                     | **3**   | ➖       | Bulk statistical files                                                       |
| **HCUP / NIS**                          | Hospital discharge statistics               | Restricted              | **3**   | ➖       | Controlled access; widely used in health services research                  |
| **ICD-11**                              | Diagnostic classification (modern)          | N/a                     | **3**   | ➖       | WHO coding system; limited API                                              |
| **OMIM**                                | Mendelian diseases                          | Restricted              | **2–3** | ➖       | Commercial license; no open API                                              |
| **Bgee**                                 | Cross-species expression                      | REST                    | **1–2** | ❌       | Anatomically-mapped expression data                                         |
| **GUDMAP / RBK**                        | Developmental expression (GU tract)         | N/a                     | **2**   | ➖       | Bulk downloads only                                                         |
| **JASPAR**                              | TF binding motifs                            | REST                    | **1–2** | ❌       | Open transcription factor motif database                                    |
| **ReMap**                               | Regulatory atlas (ChIP-seq)                 | REST                    | **1–2** | ❌       | TFBS & epigenomic peak catalogs                                            |
| **ENCODE Portal**                       | Regulatory genomics                          | REST                    | **1**   | ❌       | ChIP-seq, ATAC-seq, RNA-seq metadata & files                                |
| **Roadmap Epigenomics**                 | Epigenomic reference datasets               | N/a                     | **2**   | ➖       | Bulk metadata; no unified API                                               |
| **Addgene Sequence API**                | Plasmid sequence retrieval                   | REST                    | **2**   | ❌       | Programmatic access to deposited plasmid sequences                          |
| **Synthego CRISPR Guides**              | gRNA design & activity                       | REST                    | **1–2** | ❌       | CRISPR guide activity predictions                                            |
| **Broad GPP Perturbation Data**         | CRISPR & RNAi datasets                       | N/a                     | **2**   | ➖       | Bulk DepMap-related perturbation screens                                    |
| **Perturb-seq / CROP-seq (GEO-linked)** | Single-cell perturbation datasets            | N/a                     | **2**   | ➖       | Bulk-only; no API                                                            |
| **PDBe-KB**                             | Structural annotations around PDB            | REST                    | **1–2** | ❌       | Functional/biophysical annotations                                           |
| **ProteomeXchange**                     | Proteomics metadata aggregator               | REST                    | **1–2** | ❌       | Umbrella for PRIDE, MassIVE, PeptideAtlas                                   |
| **BioModels**                           | Systems biology models (SBML)                | REST                    | **1–2** | ❌       | Curated mathematical models                                                 |
| **JWS Online**                          | Kinetic models                                | REST                    | **1–2** | ❌       | SBML model execution & metadata                                            |
| **SwissPalm**                           | Protein palmitoylation                       | N/a                     | **2–3** | ➖       | Bulk-only; PTM database                                                     |
| **IHME GBD**                            | Global Burden of Disease                     | N/a                     | **3**   | ➖       | Bulk-only global health metrics                                             |
| **VAERS**                               | Vaccine adverse events                       | REST                    | **1–2** | ❌       | CDC/FDA open vaccine safety reporting                                       |
| **EuroStat Health API**                 | EU health & epidemiology                      | REST                    | **2–3** | ❌       | European health statistics                                                   |
| **Monarch Initiative**                  | Gene–disease–phenotype graph                  | REST                    | **1–2** | ❌       | Cross-species knowledge graph                                               |
| **BioPortal (NCBO)**                    | Biomedical ontology repository                 | REST                    | **1–2** | ❌       | Hundreds of ontologies; essential for NLP                                   |
| **Wikidata Biomedical**                 | Crowd-sourced structured biomedical graph     | REST/SPARQL             | **1–2** | ❌       | Huge linked-data knowledge graph                                            |
| **OpenBEL**                             | Biological expression language graphs         | REST                    | **1–2** | ❌       | Cause–effect biological network models                                     |
| **Bgee Expression Atlas**               | Anatomical expression across species          | REST                    | **1–2** | ❌       | Distinct from Expression Atlas (EBI)                                        |
| **Allen Brain Atlas**                   | Spatial CNS gene expression                   | REST                    | **1–2** | ❌       | ISH, RNA-seq, cell types                                                    |
| **MIMIC-III/IV Clinical Notes**          | Clinical text corpus                        | Restricted              | **3**   | ➖       | Requires credentialing; deidentified ICU notes                              |
| **i2b2 NLP Challenges**                  | Clinical NLP corpora                        | N/a                     | **3**   | ➖       | Gold-standard datasets for medication, diagnosis, de-ID tasks               |
| **n2c2 NLP Datasets**                    | Clinical text annotation                     | Restricted              | **3**   | ➖       | Successor to i2b2; requires data use agreement                              |
| **MedMentions**                          | Biomedical entity annotations                 | N/a                     | **2**   | ➖       | PubMed abstracts annotated with UMLS concepts                               |
| **BC5CDR**                               | Chemical–disease interaction corpus           | N/a                     | **2**   | ➖       | Widely used for biomedical NER & relation extraction                        |
| **CRAFT Corpus**                         | Full-text biomedical annotations              | N/a                     | **2–3** | ➖       | Ontology-grounded annotations                                               |
| **BioASQ**                               | QA over biomedical ontologies                 | N/a                     | **2–3** | ➖       | Benchmark for semantic QA                                                   |
| **SemEval BioNLP Tracks**                | Biomedical event extraction                   | N/a                     | **3**   | ➖       | Gold-standard text-mining annotations                                       |
| **PMC Open Access Subset**               | Full-text articles                            | N/a                     | **2–3** | ➖       | Bulk download; no structured REST                                           |
| **SpatialDB**                            | Spatial transcriptomics atlas                 | N/a                     | **2**   | ➖       | Aggregated spatial transcriptomics datasets                                 |
| **10x Visium Public Datasets**           | Spatial transcriptomics                       | N/a                     | **2**   | ➖       | Visium datasets; bulk-only                                                  |
| **Slide-seq / Slide-seqV2**              | Spatial barcoding datasets                    | N/a                     | **2**   | ➖       | Bulk GEO-based datasets                                                     |
| **MERFISH Public Datasets**              | Spatial single-molecule transcriptomics       | N/a                     | **2**   | ➖       | Bulk-only spatial datasets                                                  |
| **seqFISH Public Datasets**              | High-plex spatial transcriptomics             | N/a                     | **2**   | ➖       | Bulk-only access                                                            |
| **HuBMAP**                               | Human tissue atlas (spatial)                  | REST                    | **1–2** | ❌       | Spatial transcriptomics & CCF ontology                                      |
| **CRISPRbrain**                          | CRISPR screens in neurons                     | REST                    | **1–2** | ❌       | Functional genomic screens                                                  |
| **BioGRID ORCS**                         | Genome-wide CRISPR screen results             | REST                    | **1–2** | ❌       | CRISPR knockout/activation datasets                                         |
| **GenomeCRISPR**                         | CRISPR knockout phenotypes                    | N/a                     | **2**   | ➖       | Historical CRISPR screens; bulk-only                                        |
| **Project Score (Sanger)**               | CRISPR knockout screens                        | N/a                     | **2**   | ➖       | Essentiality screens complement DepMap                                      |
| **Achilles Project**                     | Historical RNAi/CRISPR essentiality            | N/a                     | **3**   | ➖       | Legacy DepMap precursor                                                     |
| **EPA CompTox / DSSTox**                 | Chemical toxicity & identifiers                | REST                    | **1–2** | ❌       | Environmental chemical safety database                                      |
| **Tox21**                                | Toxicology HTS profiles                        | N/a                     | **2**   | ➖       | Joint NIH–EPA toxicity screening                                            |
| **ADMETlab**                             | Drug ADMET predictions                         | N/a                     | **2–3** | ➖       | Bulk prediction datasets                                                    |
| **TCRD (IDG)**                           | Understudied drug targets                      | REST                    | **1–2** | ❌       | Illuminating the Druggable Genome project                                   |
| **ENCODE cCRE Registry**                 | Candidate regulatory elements                   | REST                    | **1**   | ❌       | Consolidated enhancer/promoter catalogs                                     |
| **Cistrome DB**                          | TF ChIP-seq peaks                              | REST                    | **1–2** | ❌       | Curated transcription factor & histone mark peaks                           |
| **GTRD**                                 | Transcription factor binding                    | N/a                     | **2–3** | ➖       | Aggregated ChIP-seq peak meta-collection                                    |
| **EpiMap**                               | Epigenomic atlas                                | N/a                     | **2–3** | ➖       | Large enhancer & methylation maps, bulk-only                                |
| **SCREEN (ENCODE)**                      | Enhancer atlas                                  | N/a                     | **2**   | ➖       | Bulk-only structured enhancer data                                          |
| **MyVariant.info**                       | Variant annotation aggregator                   | REST                    | **1**   | ❌       | Unified variant-level annotations                                           |
| **MyGene.info**                          | Gene annotation aggregator                      | REST                    | **1**   | ❌       | High-speed gene metadata API                                                |
| **VariantValidator**                     | HGVS validation                                 | REST                    | **1–2** | ❌       | Validates & normalizes HGVS expressions                                     |
| **RefSeqGene / LRG**                     | Stable gene loci                                | N/a                     | **2**   | ➖       | Long-term stable genomic reference loci                                     |
| **gnomAD Constraint Metrics**            | Gene constraint/LoF intolerance                  | REST                    | **1–2** | ❌       | Distinct from allele frequency endpoints                                    |
| **PhosphoSitePlus**                      | PTMs (phospho, acetyl, etc.)                    | Restricted              | **2–3** | ➖       | Gold-standard PTM database                                                  |
| **dbPTM**                                 | PTM annotations                                  | N/a                     | **2–3** | ➖       | Bulk PTM dataset                                                            |
| **iPTMnet**                               | PTM interaction networks                         | REST                    | **1–2** | ❌       | Integrates PTMs and regulatory interactions                                 |
| **DisProt**                               | Intrinsically disordered proteins                | REST                    | **1–2** | ❌       | Curated disorder annotations                                                |
| **TMHMM / Phobius**                       | Membrane topology predictions                    | N/a                     | **2–3** | ➖       | Predictor datasets; bulk-only                                               |
| **REACH (EU Chemical Safety)**            | Chemical safety                                    | Restricted              | **3**   | ➖       | No public API                                                               |
| **ECHA eChemPortal**                      | Chemical safety data                              | N/a                     | **3**   | ➖       | Bulk chemical hazard data                                                   |
| **NIH ToxRefDB**                          | Toxicology studies                                | N/a                     | **2–3** | ➖       | Historical animal toxicology data                                          |
| **Ensembl Metazoa**                       | Multispecies annotation                            | REST                    | **1**   | ❌       | Non-vertebrate Ensembl                                                       |
| **Ensembl Plants**                        | Plant genomes                                      | REST                    | **1**   | ❌       | Part of Ensembl ecosystem                                                   |
| **Ensembl Fungi**                         | Fungal genomics                                    | REST                    | **1**   | ❌       | Non-animal Ensembl                                                          |
| **Ensembl Bacteria**                      | Bacterial genomics                                 | REST                    | **1**   | ❌       | Microbial genomes metadata                                                  |
| **JGI MycoCosm**                          | Fungal genomics                                    | Restricted              | **2–3** | ➖       | Requires login; no open API                                               |
| **Virus Pathogen Resource (ViPR)**        | Viral genomics & metadata                          | REST                    | **1–2** | ❌       | Coronavirus, filovirus, flavivirus datasets                                 |
| **Influenza Research Database (IRD)**     | Flu genomics                                       | REST                    | **1–2** | ❌       | Major influenza sequence & metadata resource                                |
| **CAMI Benchmarks**                       | Metagenomics benchmarking                          | N/a                     | **3**   | ➖       | Benchmark datasets for microbiome pipelines                                 |
| **AMRFinderPlus DB**                      | AMR gene reference                                 | N/a                     | **2–3** | ➖       | Bulk AMR reference used clinically                                          |
| **Exposome-Explorer**                     | Exposure biomarkers                                | N/a                     | **2–3** | ➖       | Environmental & dietary exposure markers                                    |
| **EPA AQS**                               | Air quality system                                 | REST                    | **2**   | ❌       | Environmental exposure data                                                 |
| **EPA Envirofacts**                       | Environmental chemistry & pollution                | REST                    | **2–3** | ❌       | Chemical/toxicological environmental exposure                                |
| **DailyMed SPL**                          | Drug labeling XML                                   | REST                    | **1–2** | ❌       | Structured FDA drug labeling                                                |
| **UNII (GSRS)**                           | Ingredient identifier system                        | REST                    | **1–2** | ❌       | Global substance registration system                                        |
| **Bgee**                                  | Anatomical gene expression                          | REST                    | **1–2** | ❌       | Cross-species expression atlas                                              |
| **GUDMAP / RBK**                          | Developmental GU expression                         | N/a                     | **2–3** | ➖       | Bulk-only developmental atlas                                               |
| **JASPAR**                                | TF binding motifs                                    | REST                    | **1–2** | ❌       | Curated transcription factor motif models                                   |
| **ReMap**                                 | TF ChIP-seq regulatory atlas                        | REST                    | **1–2** | ❌       | Harmonized TF binding peak collections                                      |
| **ENCODE Portal (Regulatory)**            | Regulatory genomics datasets                        | REST                    | **1**   | ❌       | ChIP-seq, ATAC-seq, DNase-seq                                               |
| **Roadmap Epigenomics**                   | Epigenomic reference datasets                       | N/a                     | **2**   | ➖       | Bulk-only epigenome maps                                                    |
| **Synthego Guide Design API**             | CRISPR gRNA design                                   | REST                    | **1–2** | ❌       | Computational CRISPR guide predictions                                      |
| **Broad GPP Perturbation Data**           | CRISPR & RNAi screens                                | N/a                     | **2–3** | ➖       | Bulk-only gene perturbation screens                                         |
| **Perturb-seq / CROP-seq**                | Single-cell CRISPR perturbation datasets             | N/a                     | **2–3** | ➖       | Bulk GEO/SRA datasets                                                       |
| **PDBe-KB**                               | Integrated PDB annotations                           | REST                    | **1–2** | ❌       | Function & biophysical annotation layer over PDB                             |
| **ProteomeXchange**                       | Proteomics metadata aggregator                       | REST                    | **1–2** | ❌       | Umbrella for PRIDE, MassIVE, PeptideAtlas                                   |
| **BioModels**                             | Systems biology mathematical models                  | REST                    | **1–2** | ❌       | SBML models & curation                                                      |
| **JWS Online**                             | Kinetic models & simulation                          | REST                    | **1–2** | ❌       | SBML model server                                                            |
| **SwissPalm**                             | Protein palmitoylation                                | N/a                     | **2–3** | ➖       | Bulk PTM dataset                                                             |
| **IHME GBD**                              | Global Burden of Disease                              | N/a                     | **3**   | ➖       | No unified public API                                                        |
| **VAERS**                                 | Vaccine adverse events                                | REST                    | **1–2** | ❌       | CDC/FDA vaccine safety data                                                  |
| **EuroStat Health API**                   | EU health & epidemiology                               | REST                    | **2–3** | ❌       | Harmonized EU datasets                                                       |
| **Monarch Initiative**                    | Gene–disease–phenotype graph                          | REST                    | **1–2** | ❌       | Cross-species integrative biomedical KG                                      |
| **BioPortal (NCBO)**                      | Biomedical ontology repository                         | REST                    | **1–2** | ❌       | Programmatic access to hundreds of ontologies                               |
| **Wikidata Biomedical**                   | Linked-data knowledge graph                            | REST/SPARQL             | **1–2** | ❌       | Community-curated structured biomedical data                                 |

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
