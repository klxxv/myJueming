# Garden Bottom Sheet Design QA

- Source visual truth: `/var/folders/mt/_gmt3m8n7_11k6mtv2ltnh_m0000gn/T/codex-clipboard-abd36234-b710-409e-9256-c857e7335556.png`
- Implementation: `http://127.0.0.1:1420/`
- Implementation screenshot: Codex in-app Browser tab 1 capture from 2026-09-14; the capture-only browser surface did not expose a filesystem path.
- Combined comparison evidence: Codex in-app Browser tab 2 capture from 2026-09-14, showing the source and live implementation in one frame; the temporary comparison page was removed after inspection.
- Viewport: implementation 1280 × 720 CSS px; source image 571 × 776 px.
- Density normalization: no resampling used. The source is a narrow crop of the previous UI rather than a full-window target, so comparison was limited to the lower garden/status relationship and avoided pixel-level claims about unrelated regions.
- States: garden closed, garden open, and garden open with the navigation collapsed to 68 px.

## Findings

No actionable P0, P1, or P2 issues remain.

- Fonts and typography: existing application font tokens, weights, and status-bar scale are preserved. The garden's supporting copy uses the existing body and callout tokens.
- Spacing and layout rhythm: the closed garden no longer consumes a row. In the open state the 154 px sheet starts exactly at the navigation's right edge and meets the status bar with no gap. The relationship remains exact with both 150 px and 68 px navigation widths.
- Colors and visual tokens: the garden and status bar use `--surface-subtle`; hover, selected, line, and text colors reuse semantic theme variables. No hard-coded white garden surface was introduced.
- Image quality and asset fidelity: the generated cat-and-dog paw mark is a 256 × 256 RGBA PNG with preserved alpha, displayed at 34 × 34 CSS px without stretching or transparency halos.
- Copy and content: the visible `来花园走走` strip was removed. Its meaning remains as the paw button's accessible label and changes to `收起决明小花园` while open.
- Interaction and accessibility: the paw button exposes pressed state and a changing accessible label. The hidden garden is both invisible and inert. Open, close, and animal-petting interactions passed; petting no longer closes the sheet. The browser reported no console warnings or errors.

## Comparison History

1. Initial implementation — P1: assigning the overlaid garden to grid column 2 caused auto-placed main content to move into the zero-width annotation column. Evidence: main content width was 0 px and garden y-position was -79 px.
2. Fix — explicitly pinned the navigation, main stage, and annotation drawer to grid columns 1, 2, and 3. Post-fix evidence: main content width 1130 px, garden x-position 150 px, navigation right edge 150 px, garden bottom 666 px, and status-bar top 666 px.
3. Collapsed-navigation pass — navigation width and garden x-position both measured 68 px; the sheet stayed connected to the status bar and to the navigation edge.

## Implementation Checklist

- [x] Remove the garden's collapsed standalone row.
- [x] Put the navigation to the left of the garden.
- [x] Animate the garden upward from the content area's bottom edge.
- [x] Place a generated cat-and-dog paw button before the project status.
- [x] Verify open, closed, and collapsed-navigation states.
- [x] Verify that petting either animal does not close the garden.
- [x] Check browser console errors.

## Follow-up Polish

No blocking polish items remain.

final result: passed
