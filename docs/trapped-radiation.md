# Trapped radiation (AP-8 and AE-8)

The analysis `orbit.trapped_flux` gives the orbit-averaged and peak integral flux of the protons and electrons trapped in the Earth's radiation belts, and the fluence over the mission, from the
NASA AP-8 and AE-8 models. It is the environment half of a total-dose estimate: it does **not** convert flux to dose or to a depth in the shielding (that is a later stage of
[#470](https://github.com/akira-nkjm/iwafune/issues/470), and `orbit.total_dose` is unchanged).

## What it computes

1. The design orbit is propagated for one day (`spec::daily_sampling`; the step follows the orbital period, at most 10 minutes), long enough for the ground track of a low orbit to go once around the Earth.
2. At each sample the position is turned into ECEF and the McIlwain L and the ratio B/B₀ are computed (`sat_astro::radiation::belt_position`). The default (`BeltConvention::Literature`) is the literature definition: L from the map's own field at its epoch with the fixed dipole moment M = 0.311653 G·Rₑ³, and B₀ = M/L³. A comparison variant (`ModelDipoleLineMinimum`: L from the field's own dipole moment, B₀ = the minimum of |B| on the field line, as `SHELLG` of the IGRF package gives) exists for checking; the analysis uses the default.
3. For each of the four maps (protons and electrons, solar minimum and maximum) and for each energy of a standard grid, the integral omnidirectional flux above that energy (cm⁻² s⁻¹) is read from the map.
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

- NASA's sample output of the data set (`radbelt.log`): all 294 integral-flux values of the AE8MIN tables (0 and non-zero, on and off the nodes of the map, B/B₀ = 1 and > 1) agree with the printed 4 digits; the largest relative difference is 4.98·10⁻⁴, the rounding of the 4-digit table. (The earlier approximate scheme differed by up to 0.19 % for L between nodes at B/B₀ > 1.)
- The proton tables of that log are for the older compressed map AP8MAC, not AP8MAX/AP8MIN. At the energies and L ≤ 3 where the maps share nodes, AP8MAX agrees to 2.4·10⁻⁴; elsewhere the maps legitimately differ, so the log gives no complete proton check.
- **Public end-to-end point** (NASA `radbelt` README: lon −45°, lat −30°, 500 km, 2021-03-01, AP8MAX, > 20 MeV, IGRF → 2642.50 cm⁻² s⁻¹), with the same lookup and IGRF-14 at that date: L = 1.4310, B = 18997 nT, B₀ = 10635 nT, B/B₀ = 1.7863 and a flux of 3504 (+33 %) with the literature definition; L = 1.4133, B₀ = 10648 nT, B/B₀ = 1.7842 and 2636 (−0.3 %) with the comparison variant. The flux near L = 1.4 changes by about 15 % per 0.01 in L, so small differences in how L is defined are visible. The README value comes from a tool that takes L and B₀ from the IGRF routines with the field's own dipole moment for the date, which is what the comparison variant does; a test pins that agreement to 1 %. The literature definition uses a fixed moment and therefore a different L at the same point.
- These comparisons run in the tests when the data files are installed (see `crates/lib/astro/src/radiation/tests/nasa.rs`); the files are not in the repository.

## Data sources

The analysis needs seven files. They are **not** in the repository or in the program: they are installed into the per-user data cache (the same place as the DE440 ephemeris) with

```text
fune data fetch radiation                       # from the project's release (needs gh)
fune data fetch radiation --from <DIR>          # from a directory that holds the files (named as below)
fune data status                                # shows the state, the SHA-256 check and the attribution of each
```

The GUI settings page fetches with `POST /data/{id}/fetch`; `--from` is deliberately CLI-only (HTTP does not take a path on the server). The id `radiation` stands for all seven.
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

**Attribution.**

- The maps and the two old field-coefficient files: NSSDC data set PT-11B (D. Bilitza, NASA GSFC/NSSDC, March 1996), from the NASA CCMC ModelWeb archive; NASA states no licence for these files; redistributed unmodified from NASA's public archive.
  References: Sawyer & Vette 1976 (NSSDC/WDC-A-R&S 76-06), Vette 1991 (NSSDC/WDC-A-R&S 91-24), Heynderickx & Beliaev 1995 (J. Spacecraft and Rockets 32, 190–192).
- IGRF-14: International Association of Geomagnetism and Aeronomy (2024), IGRF-14, Zenodo, doi:10.5281/zenodo.14012302, CC BY 4.0; Beggan et al. 2026, doi:10.1186/s40623-025-02360-0.

The lookup procedure is that of NSSDC RADBELT, TRMFUN.FOR (1987). The file formats are described in `sat_astro::radiation::map` (the maps) and `sat_astro::geomag::legacy` (the field files and IGRF).
