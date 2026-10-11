# Pinned 5G-LENA Reference

The Phase 3 MAC reference uses 5G-LENA with ns-3. The source checkouts are
local tooling under `.tools/` and are intentionally ignored by git.

## Pins

| Component | Release tag | Peeled commit | Upstream |
|---|---|---|---|
| 5G-LENA NR module | `v5.1` | `cedceadda17392c90587fb9400eb9b1f8c236713` | https://gitlab.com/cttc-lena/nr |
| ns-3 | `ns-3.48` | `d2add90b452d600cfb4859baed8e9ea633519447` | https://gitlab.com/nsnam/ns-3-dev |

5G-LENA v5.1 documents ns-3.48 as its compatible ns-3 release. The tag object
IDs are `2fab8d9b8b5f823ab8a83495e4239b246182071b` for 5G-LENA and
`422ef9dc12d9bcd4ee8f4b1374dffcc029a3675c` for ns-3; the commit IDs above are
the values that must be used to reproduce the source trees.

## Install

```bash
mkdir -p .tools
git clone --branch ns-3.48 --depth 1 \
  https://gitlab.com/nsnam/ns-3-dev.git .tools/ns-3.48
git clone --branch v5.1 --depth 1 \
  https://gitlab.com/cttc-lena/nr.git .tools/ns-3.48/contrib/nr
cd .tools/ns-3.48
./ns3 configure --enable-examples --enable-tests
./ns3 build -j 4
```

The build requires the ns-3 prerequisites. Eigen3 is optional for the basic
NR examples but enables additional MIMO features; SQLite enables some
calibration examples.

## Reference Run

After building, inspect the available example options and run the pinned
5G-LENA demo with a fixed random seed:

```bash
./ns3 run 'cttc-nr-demo --help'
./ns3 run 'cttc-nr-demo --RngRun=1'
```

The exact scenario parameters, output traces, and command line must be stored
with any exported baseline. A generated CSV is not considered a valid Level-2
reference unless it records these details and the two commit IDs above.