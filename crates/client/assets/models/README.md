# Weapon models

One folder per weapon. Drop whatever you download into `raw/`, keep the
game-ready file in `mesh/`:

    models/<slug>/raw/      # the download, untouched -- gltf, bin, textures
    models/<slug>/mesh/     # <slug>.glb, the one weapon.rs points at

`raw/` is kept so a repack can be redone without finding the source again;
that is how the zombie lost its crawl clips once.

What `mesh_path` in crates/client/src/weapon.rs ends up as:

    mesh_path: "models/<slug>/mesh/<slug>.glb"

A `*` on the ammo means the game has no such ItemKind yet -- those six need
an ammo type added before the weapon can be loaded, so they are the last
ones worth collecting.

Already in the game: ak47_scope, m4a1, m24, ump45, deagle.

| folder | ammo | class |
|---|---|---|
| `m416` | 5.56 | AR |
| `m16a4` | 5.56 | AR |
| `scar_l` | 5.56 | AR |
| `aug_a3` | 5.56 | AR |
| `qbz95` | 5.56 | AR (Sanhok) |
| `g36c` | 5.56 | AR (Vikendi) |
| `k2` | 5.56 | AR (Taego) |
| `famas` | 5.56 | AR |
| `beryl_m762` | 7.62 | AR |
| `mk47_mutant` | 7.62 | AR |
| `ace32` | 7.62 | AR |
| `groza` | 7.62 | AR (thung do) |
| `honey_badger` | .300 BLK * | AR (thung do) |
| `mini14` | 5.56 | DMR |
| `qbu` | 5.56 | DMR (Sanhok) |
| `mk12` | 5.56 | DMR |
| `sks` | 7.62 | DMR |
| `slr` | 7.62 | DMR |
| `vss` | 9mm | DMR (ong+giam thanh lien) |
| `svd` | 7.62 | DMR (thung do) |
| `mk14_ebr` | 7.62 | DMR (thung do) |
| `kar98k` | 7.62 | SR |
| `mosin_nagant` | 7.62 | SR |
| `win94` | .45 | SR (lay) |
| `awm` | .300 Mag * | SR (thung do) |
| `lynx_amr` | .50 BMG * | SR (thung do) |
| `vector` | .45 | SMG |
| `micro_uzi` | 9mm | SMG |
| `tommy_gun` | .45 | SMG |
| `bizon` | 9mm | SMG |
| `mp5k` | 9mm | SMG (Vikendi) |
| `js9` | 9mm | SMG |
| `p90` | 5.7mm * | SMG (thung do) |
| `dp28` | 7.62 | LMG |
| `m249` | 5.56 | LMG (thung do) |
| `mg3` | 7.62 | LMG (thung do) |
| `s686` | 12g | Shotgun |
| `s1897` | 12g | Shotgun |
| `s12k` | 12g | Shotgun |
| `o12` | 12g | Shotgun |
| `dbs` | 12g | Shotgun (thung do) |
| `sawed_off` | 12g | Shotgun (o sung ngan) |
| `p18c` | 9mm | Pistol |
| `p92` | 9mm | Pistol |
| `p1911` | .45 | Pistol |
| `r1895` | 7.62 | Pistol |
| `r45` | .45 | Pistol |
| `skorpion` | 9mm | Pistol |
| `flare_gun` | phao hieu * | Pistol |
| `crossbow` | mui ten * | Nỏ |
| `panzerfaust` | dung 1 lan | Rocket |
| `mortar` | dan coi | Sung coi |

Melee (pan, machete, crowbar, sickle) is not here -- say the word and it is one command.
