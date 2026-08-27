# CIEDE2000 reference data

The integration test `tests/delta_e.rs` reads the 34-row supplementary data
from `test_data/ciede2000testdata.txt`. The file is intentionally ignored by
Git because it is external test data; the test does not embed it in the crate
source.

Source: <https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/ciede2000testdata.txt>

Expected SHA-256:

`44AEBB39107128328ADD54FBEF5AC8EE89909E50508F448A1580ADEA2058A4B8`

To restore the fixture in a fresh checkout on PowerShell:

```powershell
New-Item -ItemType Directory -Force test_data | Out-Null
Invoke-WebRequest `
  -Uri 'https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/ciede2000testdata.txt' `
  -OutFile 'test_data/ciede2000testdata.txt'
Get-FileHash 'test_data/ciede2000testdata.txt' -Algorithm SHA256
```

The ordinary test suite deliberately does not require this ignored fixture.
Set `CIEDE2000_TEST_DATA` to use a fixture outside the repository when needed.
Run the official-data gate explicitly after restoring it:

```powershell
cargo test --target-dir .test-target `
  --test delta_e sharma_supplementary_cases -- --ignored
cargo test --target-dir .test-target
```
