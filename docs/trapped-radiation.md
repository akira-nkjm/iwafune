# Trapped radiation (AP-8 and AE-8)

The analysis `orbit.trapped_flux` gives the orbit-averaged and peak integral flux of the protons and electrons trapped in the Earth's radiation belts, and the fluence over the mission, from the
NASA AP-8 and AE-8 models. It is the environment half of a total-dose estimate. The conversion to dose behind aluminium shielding is the analysis `orbit.trapped_dose` (see [Dose behind shielding](#dose-behind-shielding-orbittrapped_dose)),
which `orbit.total_dose` uses when a leg has no environment table of its own ([#470](https://github.com/akira-nkjm/iwafune/issues/470)).

## What it computes

1. The design orbit is propagated for one day (`spec::daily_sampling`; the step follows the orbital period, at most 10 minutes), long enough for the ground track of a low orbit to go once around the Earth.
2. At each sample the position is turned into ECEF and the McIlwain L and the ratio B/B₀ are computed (`sat_astro::radiation::belt_position`). The default (`BeltConvention::Literature`) is the literature definition: L from the map's own field at its epoch with the fixed dipole moment M = 0.311653 G·Rₑ³, and B₀ = M/L³. A comparison variant (`ModelDipoleLineMinimum`: L from the field's own dipole moment, B₀ = the minimum of |B| on the field line, as `SHELLG` of the IGRF package gives) exists for checking; the analysis uses the default.
3. For each map (protons and electrons, solar minimum and maximum) and for each energy of a standard grid, the integral omnidirectional flux above that energy (cm⁻² s⁻¹) is read from the map.
   The grids are 0.1 to 400 MeV for protons and 0.04 to 7 MeV for electrons (`proton_energies_mev`, `electron_energies_mev`).
4. The output, per map and per energy, is the time-averaged flux, the peak flux, the fraction of time with a non-zero flux, and the **fluence** = time-averaged flux × the mission duration
   (the same mission duration that `orbit.total_dose` uses when the design has no phases). The one-day sample is taken to represent the whole mission; the orbit's evolution over the mission is not followed.

Samples where L cannot be found (a field line that does not close within 16 Earth radii, as at high latitude) are outside the model: they count as zero flux and are listed in `samples_without_l`.

| Analysis id | `orbit.trapped_flux` (area: orbit conditions, kind: steady) |
|---|---|
| Needs | the orbit (Earth only), the mission duration, and the data below installed (otherwise the analysis is a missing input that says how to install it) |
| Does not need | any part data |

## Models and the field each is paired with

A map is a table built in a specific geomagnetic field and must be read with the same field. B and L are computed in that field **at the model's own epoch, without secular update**.

| Map | Particle | Solar activity (epoch) | Field |
|---|---|---|---|
| AP8MIN (`ap8min`) | protons | minimum (1964) | Jensen & Cain (1960) |
| AP8MAX (`ap8max`) | protons | maximum (1970) | GSFC 12/66, extrapolated to 1970 |
| AE8MIN (`ae8min`) | electrons | minimum (1964) | Jensen & Cain (1960) |
| AE8MAX (`ae8max`) | electrons | maximum (1970) | Jensen & Cain (1960) |

This is the pairing used by SPENVIS and IRBEM. The NASA readme says the maps should be used with a field of the model epoch (1964 and 1970); the common practice above is what the
published implementations do, and it is what the NASA sample output was produced with.

The old coefficient files are converted into the Schmidt semi-normalised convention of `sat_astro::geomag::GaussModel` (`geomag::legacy`):

- **GSFC 12/66**: already Schmidt, with the usual sign. The coefficient of the year t is `g + ġ·Δt + ½·g̈·Δt²` (Δt = t − 1960). Whether the file's second derivative is meant with the ½ is **not stated**, so this is an assumption; the second-derivative term is about 3 nT of 30000 nT for the dipole at 1970, so the choice changes the field by about 1e-4.
- **Jensen & Cain 1960**: the sign is the opposite of IGRF (`g₁⁰ = +30411.2 nT`) and the Legendre functions are the Chapman–Bartels "Gauss" functions. The conversion is
  `g' = −g·(n−m)! / ((2n−1)!!·Sₙₘ)` with `Sₙₘ = √((2−δₘ₀)(n−m)!/(n+m)!)` (the same for h). It was checked against IGRF-14's coefficients of 1960: all coefficients up to degree 4 agree within 120 nT.
- In both files the degree and order in a row are the real values plus one.

## Limits

- The models are **static**: one map for solar minimum (1964) and one for solar maximum (1970). There is no solar-cycle phase, no variation of the belts with magnetic storms, and no secular drift of the field.
- **No solar energetic particles** and no galactic cosmic rays. Only trapped protons and electrons.
- The AE-8 electron model is known to be inaccurate for the outer belt and for the newest observations (AE9/AP9 are the successors); AP-8 is known to overestimate the low-energy proton flux at solar maximum in the South Atlantic Anomaly region.
  The numbers are for concept design.
- The lookup follows NASA's TRARA1/TRARA2 step for step (single precision, truncated scaled L and B/B₀, integer FISTEP; see `sat_astro::radiation::flux`). Near the atmospheric cut-off of the proton belt (large B/B₀ at low L, i.e. low orbits) the flux is very sensitive to L and B/B₀, so the (L, B/B₀) definition matters by tens of percent (see Verification).
- An orbit about another body is not supported.

## Verification

The lookup is a re-implementation of the procedure of the NSSDC RADBELT program (TRMFUN.FOR, 1987: subroutines TRARA1 and TRARA2), written from reading that source; no code or text of it is in this repository.

- NASA's sample output of the data set (`radbelt.log`): every integral-flux value of the AE8MIN tables that the test uses (zero and non-zero, on and off the nodes of the map, B/B₀ = 1 and > 1) agrees with the printed 4 digits; the largest relative difference is 4.98·10⁻⁴, the rounding of the 4-digit table. (The earlier approximate scheme differed by up to 0.19 % for L between nodes at B/B₀ > 1.)
- The proton tables of that log are for the older compressed map AP8MAC, not AP8MAX/AP8MIN. At the energies and L ≤ 3 where the maps share nodes, AP8MAX agrees to 2.4·10⁻⁴; elsewhere the maps legitimately differ, so the log gives no complete proton check.
- **Public end-to-end point** (NASA `radbelt` README: lon −45°, lat −30°, 500 km, 2021-03-01, AP8MAX, > 20 MeV, IGRF → 2642.50 cm⁻² s⁻¹), with the same lookup and IGRF-14 at that date: L = 1.4310, B = 18997 nT, B₀ = 10635 nT, B/B₀ = 1.7863 and a flux of 3504 (+33 %) with the literature definition; L = 1.4133, B₀ = 10648 nT, B/B₀ = 1.7842 and 2636 (−0.3 %) with the comparison variant. The flux near L = 1.4 changes by about 15 % per 0.01 in L, so small differences in how L is defined are visible. The README value comes from a tool that takes L and B₀ from the IGRF routines with the field's own dipole moment for the date, which is what the comparison variant does; a test pins that agreement to 1 %. The literature definition uses a fixed moment and therefore a different L at the same point.
- These comparisons run in the tests when the data files are installed (see `crates/lib/astro/src/radiation/tests/nasa.rs`); the files are not in the repository.

## Data sources

The analyses need the files listed below. They are **not** in the repository or in the program: they are installed into the per-user data cache (the same place as the DE440 ephemeris) with

```text
fune data fetch radiation                       # from the project's release (needs gh)
fune data fetch radiation --from <DIR>          # from a directory that holds the files (named as below)
fune data status                                # shows the state, the SHA-256 check and the attribution of each
```

The GUI settings page fetches with `POST /data/{id}/fetch`; `--from` is deliberately CLI-only (HTTP does not take a path on the server). The id `radiation` stands for the whole group.
Each file's size and SHA-256 are checked before it replaces anything.

| id | file | what |
|---|---|---|
| `ap8min` | `ap8min.asc` | AP-8 MIN trapped proton map |
| `ap8max` | `ap8max.asc` | AP-8 MAX trapped proton map |
| `ae8min` | `ae8min.asc` | AE-8 MIN trapped electron map |
| `ae8max` | `ae8max.asc` | AE-8 MAX trapped electron map |
| `jensen_cain_1960` | `jensen_cain_62.dat` | Jensen & Cain (1960) field coefficients |
| `gsfc_1966` | `gsfc66.dat` | GSFC 12/66 field coefficients |
| `igrf14` | `igrf14coeffs.txt` | IGRF-14 (the present-day field; not used by this analysis yet) |
| `shieldose2` | `shieldose2.dat` | SHIELDOSE-2 dose data bases (release tag `data-shielding-v1`; only `orbit.trapped_dose` and the computed environment of `orbit.total_dose` need it) |

**Attribution.**

- The maps and the two old field-coefficient files: NSSDC data set PT-11B (D. Bilitza, NASA GSFC/NSSDC, March 1996), from the NASA CCMC ModelWeb archive; NASA states no licence for these files; redistributed unmodified from NASA's public archive.
  References: Sawyer & Vette 1976 (NSSDC/WDC-A-R&S 76-06), Vette 1991 (NSSDC/WDC-A-R&S 91-24), Heynderickx & Beliaev 1995 (J. Spacecraft and Rockets 32, 190–192).
- `shieldose2.dat`: SHIELDOSE-2 dose data bases (PROTBAS2, ELBRBAS2), S. M. Seltzer, NIST, NISTIR 5477 (1994); values extracted unchanged from the IRBEM library (github.com/PRBEM/IRBEM, `prot_and_elbr_bas2.f`, LGPL-3.0-or-later), including its 2013 correction of the bremsstrahlung tables. The file is one plain-text file (format in `sat_astro::radiation::shielding::data`) made by a one-off conversion of the Fortran DATA statements; only the exponent marker `D` was changed to `e`.
- IGRF-14: International Association of Geomagnetism and Aeronomy (2024), IGRF-14, Zenodo, doi:10.5281/zenodo.14012302, CC BY 4.0; Beggan et al. 2026, doi:10.1186/s40623-025-02360-0.

The lookup procedure is that of NSSDC RADBELT, TRMFUN.FOR (1987). The file formats are described in `sat_astro::radiation::map` (the maps) and `sat_astro::geomag::legacy` (the field files and IGRF).

## Dose behind shielding (`orbit.trapped_dose`)

`orbit.trapped_dose` turns the orbit-averaged AP-8/AE-8 spectra into the **total ionising dose rate in silicon behind aluminium shielding** versus the shielding thickness,
for solar minimum (AP8MIN + AE8MIN) and maximum (AP8MAX + AE8MAX), split into trapped protons, trapped electrons, electron bremsstrahlung and the total. The method is SHIELDOSE-2
(S. M. Seltzer, NIST, NISTIR 5477, 1994; the first version is NBS Technical Note 1116, 1980), the model that SPENVIS and similar tools use.

| Analysis id | `orbit.trapped_dose` (area: orbit conditions, kind: steady) |
|---|---|
| Needs | the orbit (Earth only) and the data files above (a missing one is a missing input that says `fune data fetch radiation`) |
| Output | `cases[]` (solar minimum, maximum) each with a table (aluminium thickness → protons, electrons, bremsstrahlung, total dose rate), and `used` (everything below) |
| Defaults (inputs are optional and the output says which were used) | detector silicon; geometry centre of a solid aluminium sphere; no nuclear attenuation of protons; thicknesses 0.1, 0.2, 0.5, 1, 1.5, 2, 3, 4, 5, 7, 10, 15, 20 mm |

### Method

1. **Orbit positions** are those of `orbit.trapped_flux` (one day of the orbit; the (L, B/B₀) of each sample are found once and reused).
2. **Integral spectrum.** For each map the integral flux `F(>E)` is read at every sample on a log-spaced grid (25 points per decade: 0.1–400 MeV for protons, 0.04–7 MeV for electrons) and averaged over time (trapezoid).
3. **Differential spectrum** (what SHIELDOSE-2 needs): `j = (F_i − F_{i+1}) / (E_{i+1} − E_i)` between neighbouring grid points, placed at their geometric mean energy. The ratio of neighbouring energies is 1.10, so the difference from the arithmetic midpoint is below 0.1 %.
   At the **low-energy end** ln j is extended along a straight line in ln E to the first energy (otherwise the half interval below the first midpoint would lose particles). At the **high-energy end**, where F reaches 0, only the run of positive differences is used (ln j needs j > 0); the interval in which F falls to 0 gets the average `F_i / ΔE`.
   A test with an exact power law shows the differences within 0.2 %; for the synthetic map used in the analysis test the dose computed from the differenced spectrum differs by 0.02 % from the dose of the exact differential spectrum.
4. **SHIELDOSE-2** (`sat_astro::radiation::shielding`): ln j is interpolated by a natural cubic spline in ln E (no extrapolation beyond half a grid step), a log-spaced energy grid (up to 1001 nodes) is integrated with Simpson's rule (the procedure's `INTEG`), the dose kernels (tables of dose versus depth scaled by the CSDA range)
   are interpolated by spline in energy and depth for protons, and linearly in ln T for electrons and bremsstrahlung (version 2), the detector/aluminium ratios convert aluminium to silicon, and the unit constant is 1.6021892·10⁻¹⁰ Gy g/MeV. Depth in g/cm² is thickness × 2.70 g/cm³.
5. **Sphere.** For the centre of a solid sphere the dose is computed from the slab curve `D(z)` as `D (1 − dlnD/dlnz)` (straight-ahead approximation) as SHIELDOSE-2 does.

### Geometry and the factor 2

The slab results of SHIELDOSE-2 are for radiation incident on **one side** (an omnidirectional flux φ through a plane gives a rate φ/4 of crossings and an average path factor 2, so the dose at zero depth is φ/2 × stopping power).
The sphere is irradiated from **all directions**: at zero depth the dose is φ × stopping power. SHIELDOSE-2 derives the sphere value from the slab curve and prints it under the heading *"1/2 dose at the centre of aluminium spheres"*, i.e. in the slab normalisation,
half of the real dose. The kernel therefore returns both: `GeometryDoses::sphere_half` (the value SHIELDOSE-2 prints) and `sphere_centre()` (= 2 ×, the real dose). **The analysis tables use the real dose** (`geometry = sphere_centre`).
If you compare with a tool that prints SHIELDOSE-2's sphere value unchanged, divide this analysis' sphere values by 2 (or select `semi_infinite_slab`, whose normalisation equals the printed slab value).
The kernel test (see Verification) confirms the slab normalisation (φ/2 × stopping power at near-zero depth) and that the printed sphere value equals the slab value there; the factor 2 for the real sphere then follows from the geometry argument (φ × stopping power at zero depth). It was **not** read from the original report (NISTIR 5477), which could not be read from the sources available while writing this, so check it against the report when you can.

### Nuclear attenuation options

SHIELDOSE-2's `INUC` options are all implemented: `none` (1, default: protons lose energy by ionisation only, conservative at large depth), `local` (2: the proton fluence is attenuated by nuclear interactions and the charged secondaries' energy is deposited where they are made) and
`local_with_neutrons` (3: option 2 plus a crude exponential transport of the energy carried by neutrons). Options 2 and 3 are less tested here than option 1; option 2 can give more dose than option 1 at depth.

### Limits

- Trapped protons, trapped electrons and bremsstrahlung **only**: no solar energetic particles, no galactic cosmic rays, no particles produced in the spacecraft structure beyond SHIELDOSE-2's own treatment.
- Aluminium shielding in simple geometries (sphere centre and slabs); the real mass distribution around a part is not modelled. The sphere formula is the straight-ahead approximation and is not exact where scattering matters.
- The AP-8/AE-8 limits above apply (static models, no outer-belt dynamics). The solar minimum and maximum maps differ also in the field they are defined in.
- The spectrum is the **orbit-averaged** spectrum of one day; the rate is a mean over the mission and the orbit's evolution is not followed.

### Connection to `orbit.total_dose`

When a leg of `orbit.total_dose` has **no** table (`dose_rate_by_shielding`) and the data of `fune data fetch radiation` are installed, the leg's environment is computed by `orbit.trapped_dose` for **that leg's orbit** (a phase with its own `[orbit]` gets its own computation). At each thickness the table takes the **larger** of the solar minimum and maximum totals (protons dominate at minimum, electrons at maximum in low orbits, so neither condition is always the worse; the envelope is the conservative choice), using the default geometry (centre of a solid aluminium sphere) and silicon.
A user table **always wins**. Each leg's source is in the output (`legs[].environment_source`: `user_table` or `computed_trapped`), and a computed table adds a warning check `environment_source`: it covers trapped particles only, so the totals are lower bounds.
Legs without a duration are not computed. If a computation is impossible (other central body, data missing) the leg has no table and is left out as before. Without the data files the behaviour is exactly as before, and the missing-input reason also names `fune data fetch radiation`.

### Verification (SHIELDOSE-2 part)

- **Reading**: every DATA statement of the source was compared with the converted file by an independent script (array order: last index fastest; detector and the two `m` tables of the electron and bremsstrahlung arrays); the tests pin representative values and the reader rejects wrong counts, names, dimensions, missing `end` and non-numbers.
- **Near-zero depth, single energy** (the dose per fluence of the kernel against the range table of the same data, `S = dE/dR`, dose = φ/2 × S × 1.6021892·10⁻¹⁰): energies from 0.1 MeV to 1 GeV agree within 0.7 %. At 100 MeV `S` from the table is 5.65 MeV cm²/g, and the literature value for protons in aluminium is about 5.7 (ICRU Report 49, NIST PSTAR); the surface dose of a 99–101 MeV band is 0.7 % above φ/2 × S.
- **Range**: protons whose CSDA range is below the shield depth give exactly zero slab and sphere dose (10 MeV protons, range ≈ 0.17 g/cm², behind 1 g/cm²); electrons stop in a thick shield while bremsstrahlung remains.
- **Sphere from slab**: for a power-law slab curve `D = A z^−p` the sphere value is `D (1 + p)` (exact to 1e-12); for a curve quadratic in ln z it agrees with the analytic derivative within 0.1 % inside the grid.
- **Spectrum integration** (NBS TN 1116 appendix C sample, exponential solar-proton spectrum E₀ = 26.5 MeV, C = 2.45·10¹⁰, limits 2–1000 MeV): the integral spectrum is 2.2719·10¹⁰ (printed 2.2719E+10) for 201, 501 and 1001 points, and the mean energy is 28.5000 MeV. The report prints 28.50006; the exact value for this exponential is 28.5 and the printed 28.50006 differs by 2·10⁻⁶ in relative terms (the sample's grid size is not known here; the same Simpson rule gives between 28.4965 and 28.5001 for 16–30 points).
- **Analysis**: for a synthetic proton map (flux known in closed form) the sphere dose at the smallest depth is within 0.11 % of ∫ j S r dE × 1.6021892·10⁻¹⁰ Gy g/MeV computed independently (r = the silicon/aluminium ratio of the data).
- Not done: a run of the original Fortran program (no Fortran compiler was available), a comparison with SPENVIS output, and any check of nuclear attenuation options 2 and 3 beyond their ordering.
