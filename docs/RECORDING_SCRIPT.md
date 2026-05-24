# Recording script — Whistleblower demo

Designed for a ~60-second screen recording (Loom, QuickTime, OBS) that you can embed
in the README and link from the LP-0017 submission PR. The web demo's "Run guided tour"
button drives the visual flow automatically; your narration explains the architecture
while it runs.

**Tools:** Loom (recommended — produces a hosted URL automatically) or QuickTime
(produces an .mp4 you upload elsewhere).

**Setup before recording:**
1. Deploy to Vercel and open the production URL — *not* localhost. The URL bar should
   show `https://whistleblower-warfield2016.vercel.app` (or your custom domain).
2. Browser at 1280×720 or 1440×900. Zoom level 100%. Light system theme so the dark
   demo pops.
3. Disable notifications (System Settings → Focus → Do Not Disturb).
4. Close any browser tabs / extensions that show up in the chrome.
5. Click **reset demo state** if any prior runs are visible.

---

## The script (~60 seconds)

### 0:00 — 0:10 · Hook + context

> "This is Whistleblower — a censorship-resistant document publication app for the
> Logos Network. It demonstrates a three-layer architecture you can try right now in
> your browser."

*[on screen: scroll slowly from the hero down to the three empty layer panels (Codex /
Waku / LEZ chronicle-registry), then scroll back up]*

### 0:10 — 0:18 · The button click

> "I'll click 'Run guided tour' and walk you through what happens."

*[click the green ▶ Run guided tour button. The lime banner appears at the top:
"Publisher: uploading file to Codex storage"]*

### 0:18 — 0:30 · Layer 1 — storage

> "First, the publisher uploads a document. The bytes go to Codex — distributed,
> content-addressed storage — and we get back a CID. That's the violet panel filling
> in. The publisher's job is done."

*[on screen: Codex storage panel populates with "Vendor exposure review (sample)" entry.
Activity log shows `[1/4] PUBLISHER uploads bytes to Codex...` then `got CID: z...`]*

### 0:30 — 0:38 · Layer 2 — delivery

> "Next, an envelope describing the document broadcasts on a Waku pub-sub topic.
> Anyone subscribed to this topic sees it in real time. This is how the document
> becomes immediately discoverable."

*[on screen: banner changes to "Broadcaster: envelope published on Waku topic". Sky-blue
panel populates. Activity log shows the topic name]*

### 0:38 — 0:50 · Layer 3 — the key insight

> "And here's the key insight. A **third party** — anyone watching the topic — picks
> up the CID and anchors it to the LEZ chronicle-registry on-chain. The publisher does
> not need to be online, does not need tokens, does not need to coordinate with anyone.
> This is the censorship-resistance property: the publisher and the anchorer can be
> completely different actors."

*[on screen: lime banner reads "Third party: picks up the broadcast and anchors it
on-chain". Lime panel populates. Activity log highlights "the publisher did NOT need
to be online or hold tokens"]*

### 0:50 — 0:58 · Wrap

> "Now anyone, including the original publisher, can query the registry by CID and
> verify the document's existence at that timestamp. That's the full pipeline —
> upload, broadcast, anchor — running in your browser as WebAssembly, with the same
> wire format as the production Basecamp module on Logos."

*[on screen: banner clears, lookup result populates with the registry entry, activity
log shows the [done] line]*

### 0:58 — 1:00 · CTA

> "Source on GitHub, link in description."

*[on screen: click the GITHUB → link in the header to show the repo briefly, then end]*

---

## Why this script

- **Names the personas explicitly** — Publisher / Broadcaster / Third party / Anyone.
  These map to the four banner steps and to the prize spec's "permissionless" language.
- **Explains the 'why' before showing the 'how'** for the third-party anchor step. This
  is the architectural insight evaluators care about most — make sure it lands.
- **Avoids on-chain terminology jargon** in the first 30 seconds. Save "registry",
  "anchor", "tx" for after the viewer has the mental model.
- **Ends with a CTA** to the repo. The GitHub link in the hero is positioned top-right
  for exactly this moment.

## Embedding the recording

After uploading:

1. **Loom:** copy the share URL, edit `web/app/page.tsx` Hero section to add a "Watch
   demo" link next to "Run guided tour".
2. **YouTube/Vimeo:** same idea but embed a thumbnail link rather than playing inline
   (faster page load).
3. **MP4 in repo:** *don't* commit > 5MB. Use git-lfs or host externally.
4. **README:** add a `[![demo](thumbnail.png)](recording-url)` markdown image-link in
   the top section, above the "What this is" header.

## What to do if something goes wrong on the take

- **WASM didn't load:** activity log says "WASM module loaded — ready" on a successful
  page load. If it's missing, hit reload and wait 2 seconds before clicking the button.
- **Tour completes too fast for narration:** record the tour without narration first,
  then re-record narration as a voice-over track over the same clip.
- **Lookup result panel doesn't open:** the WASM `lookupJson` is synchronous so this
  only fails if the CID never anchored. Re-run from a reset.

## After recording

1. Update `web/app/page.tsx` Hero to add a "▶ Watch the demo" link to the recording.
2. Update root `README.md` to embed the recording (image-link in the top section).
3. Add the recording URL to `docs/DEPLOY.md` "After deploy" checklist.
4. Mention the recording in the LP-0017 submission PR description.
