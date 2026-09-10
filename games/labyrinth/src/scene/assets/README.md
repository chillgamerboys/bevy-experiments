# Labyrinth prototype art

Generated with the built-in image-generation tool. These are original prototype assets, not Darkest Dungeon assets. The actor sheet uses a fixed 4×2 atlas. Layout and hit regions remain independent of its pixels.

Final project assets: [actors.png](actors.png) and [room.png](room.png).
The transparent atlas retains its original generated alpha; catalog-specific source
insets remove neighboring-cell fragments at render time without modifying the file.

## Actor atlas prompt

Use case: stylized-concept. Asset type: a production 2D game character sprite atlas PNG with genuine transparent alpha background. Create ONE precisely aligned sheet 4 columns x 2 rows, each equal rectangular cell, all eight full-body characters contained within their own cell with generous transparent margins, feet on same baseline within each row, no overlaps, no labels, no text, no floor/background. Top row left to right: 1 armored gatekeeper with large shield, 2 hooded knifehand with short knives and rust scarf, 3 green-cloaked scout with bow, 4 pale-cloaked field medic with satchel and lantern. All top characters face RIGHT in three-quarter side combat stance. Bottom row left to right: 1 hulking ash-gray brute with crude club, 2 heavy iron-armored brute, 3 lean sinister wound stalker with hooked blades, 4 skeletal hollow archer. All bottom enemies face LEFT in three-quarter side combat stance. Style: original eerie labyrinth fantasy, minimal illustrated ink cutouts, bold readable silhouettes, flat muted charcoal/bone/steel/moss palettes with sparse tarnished-gold or rust accents, restrained shading, not pixel art, not photorealistic, not cartoon cute. Tall narrow full-body figures easy to read when rendered at 90px wide. Critical atlas organization: 4 equally spaced columns and 2 equally spaced rows, every figure centered in its individual cell, consistent scale and consistent bottom foot alignment. Transparent background, no checkerboard painted in, no rectangular frames, no UI, no lettering, no logos. Suggested canvas 1536x1024.

## Transparency edit prompt

Use case: background-extraction. This is a game sprite atlas edit. Remove ALL of the dark brown/black gradient backdrop and replace it with genuinely transparent alpha, including between all limbs, cloaks, weapons, and every cell. Do NOT leave any colored background rectangle, checkerboard, smoke, glows or floor. Preserve the same 4-column x 2-row atlas placement and canvas dimensions, all eight characters' positions, scale, shapes, colors, facing directions and clear margins. Keep all character interiors intact. Output RGBA PNG transparency, not simulated transparency. Characters must remain isolated cutouts suitable for compositing over a dungeon world.

## Room prompt

Use case: stylized-concept. Asset type: original side-view 2D game environment backdrop for Labyrinth, an eerie cooperative positional combat game. Wide landscape image, a deserted ancient stone maze chamber, weathered pillars and giant hexagonal doorway in the central distance, thin green-gray fog, cracked stone ground plane along bottom quarter. Quiet restrained illustrated ink-and-paint cutout art, muted charcoal, slate, gray moss, tiny tarnished gold distant lanterns, subtle depth, no excessive texture noise. Broad empty horizontal foreground for twelve character sprites added by the game. Side-on theatrical composition, not isometric or top down, clear depth and scale but no dramatic perspective tilting the ground. Mostly dark low contrast background with recognizable silhouettes, brighter mist in center behind characters, blackened upper vaults. No characters, no enemies, no text, no interface, no panels, no border. Landscape 1536x1024.

## Two-rank cutouts

Generated with the built-in image-generation tool using `actors.png` as a style
reference only. Saved as [lantern-wagon.png](lantern-wagon.png) and
[ossuary-hauler.png](ossuary-hauler.png), with real alpha transparency verified.
These standalone images do not change the original atlas or define gameplay footprints.

### Lantern Wagon prompt

Use case: stylized-concept. Asset type: transparent 2D side-view game character cutout for Labyrinth. Primary request: a single Lantern Wagon, a weathered wooden supply cart with a hooded living keeper seated on its front right, small hanging amber lantern and tied provisions. Side view facing RIGHT, whole wagon including both wheels visible, broad silhouette, fits a two-rank combatant. Style match the supplied reference characters: gritty ink outlines, angular painterly shading, desaturated dark fantasy earthy cloth and worn wood, eerie mood, readable at small size. Input image is STYLE REFERENCE ONLY, do not reproduce the sheet or other characters. Center single wagon with small even transparent margins. Genuinely transparent background with alpha, no backdrop, floor, shadow plate, text, labels, borders, or UI. Landscape canvas.

Final transparency edit: Background extraction edit. Remove the entire checkerboard background and make genuinely transparent alpha PNG, not a rendered checkerboard. Preserve the wagon and keeper exactly. No colored background, no pattern, no gradients, no floor. Keep all silhouette and interior opaque. Output isolated game cutout with real alpha transparency.

### Ossuary Hauler prompt

Use case: stylized-concept. Asset type: transparent 2D side-view game monster cutout for Labyrinth. Primary request: one massive Ossuary Hauler, hunched pale dungeon scavenger with long heavy forelimbs and an enormous iron cage of old bones strapped to its broad back. Menacing low head, chunky hands on ground, enormous heavy silhouette, facing LEFT toward the heroes. Style match supplied reference characters: gritty black ink contours, angular painterly shading, muted dark fantasy bone grey, worn iron and earthy wraps. Input image STYLE REFERENCE ONLY, do not reproduce the sheet or other figures. Full body centered with small even margins, landscape proportions, spans two formation ranks. Genuinely transparent background with alpha, no scene, floor, shadow plate, text, labels, borders, or UI.
