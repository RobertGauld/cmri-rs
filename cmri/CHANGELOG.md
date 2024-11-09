# CMRI Changelog

* Uses thiserror crate instead of a nostd fork.

## 0.1.0 ⇒ 0.1.1

* Addition of more const functions:
  * `NodeSort::try_new_smini`
  * `NodeSort::try_new_cpnode`
  * `NodeSort::try_new_cpmega`
  * `CpnodeConfiguration::try_new`
  * `CpmegaConfiguration::try_new`
  * `SminiConfiguration::try_new`
  * `SminiConfiguration::get_oscillating_pairs_count`
