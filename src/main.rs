//! Integration-style tests that fetch BRCA2 metadata from NCBI and
//! assert that key fields are parsed correctly.

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod ncbi_tests;

#[cfg(test)]
mod ensembl_tests;

#[cfg(test)]
mod uniprot_test;

#[cfg(test)]
mod gencode_test;

#[cfg(test)]
mod obo_test;

#[cfg(test)]
mod gnomad_test;

#[cfg(test)]
mod ucsc_test;

#[cfg(test)]
mod rcsb_test;

#[cfg(test)]
mod ena_test;

#[cfg(test)]
mod gdc_test;

#[cfg(test)]
mod dbsnp_test;

#[cfg(test)]
mod dbvar_test;

#[cfg(test)]
mod go_test;

#[cfg(test)]
mod hpo_test;

#[cfg(test)]
mod cptac_test;

#[cfg(test)]
mod gtex_test;

