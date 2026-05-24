"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// The WASM module is built by `npm run wasm` (which shells out to wasm-pack) before
// `next dev` / `next build`. We import it dynamically so SSR doesn't try to load
// the .wasm at module-eval time.
type WasmApi = {
  publishFileJson: (req: string, bytes: Uint8Array) => any;
  anchorBatchJson: (req: string) => any;
  lookupJson: (req: string) => any;
  listPublishedJson: () => any;
  listDeliveryLogJson: () => any;
  listAnchoredJson: () => any;
  resetDemoState: () => void;
  getTopic: () => string;
};

type PublishedRecord = {
  publish_id: string;
  envelope: Envelope;
  metadata_hash: string;
  anchored: boolean;
  anchor_tx: string | null;
};

type Envelope = {
  cid: string;
  title: string;
  description: string;
  content_type: string;
  size_bytes: number;
  timestamp: number;
  tags: string[];
};

type RegistryEntry = {
  cid: string;
  metadata_hash: number[];
  anchor_timestamp: number;
};

export default function Page() {
  const [wasm, setWasm] = useState<WasmApi | null>(null);
  const [wasmError, setWasmError] = useState<string | null>(null);

  // Form state
  const [file, setFile] = useState<File | null>(null);
  const [title, setTitle] = useState("Q3 internal memo");
  const [description, setDescription] = useState("Demo document for the publication pipeline.");
  const [contentType, setContentType] = useState("text/plain");
  const [tags, setTags] = useState("demo, whistleblower");
  const [broadcast, setBroadcast] = useState(true);

  // Live state from the WASM module
  const [published, setPublished] = useState<PublishedRecord[]>([]);
  const [deliveryLog, setDeliveryLog] = useState<Envelope[]>([]);
  const [registry, setRegistry] = useState<RegistryEntry[]>([]);

  const [lookupCid, setLookupCid] = useState("");
  const [lookupResult, setLookupResult] = useState<any>(null);

  const [statusLog, setStatusLog] = useState<string[]>([]);
  const log = useCallback((line: string) => {
    setStatusLog((prev) => [`${new Date().toLocaleTimeString()} · ${line}`, ...prev].slice(0, 50));
  }, []);

  const [tourRunning, setTourRunning] = useState(false);
  const [tourStep, setTourStep] = useState<string | null>(null);

  // Load WASM
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // @ts-ignore — generated module, types are best-effort
        const mod = await import("../lib/pkg/web_demo.js");
        await mod.default();
        if (cancelled) return;
        setWasm(mod as unknown as WasmApi);
        log("WASM module loaded — ready");
      } catch (e: any) {
        const msg = e?.message ?? String(e);
        setWasmError(msg);
        log(`WASM load failed: ${msg}`);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [log]);

  const refresh = useCallback(() => {
    if (!wasm) return;
    setPublished(wasm.listPublishedJson() ?? []);
    setDeliveryLog(wasm.listDeliveryLogJson() ?? []);
    setRegistry(wasm.listAnchoredJson() ?? []);
  }, [wasm]);

  const onPublish = useCallback(async () => {
    if (!wasm || !file) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    const req = {
      title,
      description,
      content_type: contentType,
      tags: tags.split(",").map((t) => t.trim()).filter(Boolean),
      broadcast,
    };
    const resp = wasm.publishFileJson(JSON.stringify(req), bytes);
    if (resp?.ok) {
      log(`published cid=${resp.cid.slice(0, 16)}… (${file.size} bytes)`);
    } else {
      log(`publish failed: ${resp?.error ?? "unknown"}`);
    }
    refresh();
  }, [wasm, file, title, description, contentType, tags, broadcast, log, refresh]);

  const onAnchorAll = useCallback(() => {
    if (!wasm) return;
    if (deliveryLog.length === 0) {
      log("nothing on the topic to anchor");
      return;
    }
    // For each envelope we've seen, ask the WASM to compute the same hash from the
    // canonical wire form. Easiest: re-publish their hashes through the published-record
    // list (we stored hash_wire alongside each PublishedRecord).
    // Here we use the published-records' hashes which we know match the envelopes.
    const entries = published.map((r) => ({
      cid: r.envelope.cid,
      metadata_hash: r.metadata_hash,
    }));
    if (entries.length === 0) {
      log("no published records to anchor");
      return;
    }
    const resp = wasm.anchorBatchJson(JSON.stringify({ entries }));
    if (resp?.ok) {
      log(
        `anchored batch tx=${resp.tx_hash} (new=${resp.anchored_cids?.length ?? 0}, skipped=${
          resp.skipped_duplicate_cids?.length ?? 0
        })`,
      );
    } else {
      log(`anchor failed: ${resp?.error ?? "unknown"}`);
    }
    refresh();
  }, [wasm, deliveryLog, published, log, refresh]);

  const onLookup = useCallback(() => {
    if (!wasm) return;
    const resp = wasm.lookupJson(JSON.stringify({ cid: lookupCid }));
    setLookupResult(resp);
    if (resp?.ok) {
      log(`lookup ${lookupCid.slice(0, 16)}… → ${resp.entry ? "FOUND" : "NOT IN REGISTRY"}`);
    } else {
      log(`lookup failed: ${resp?.error ?? "unknown"}`);
    }
  }, [wasm, lookupCid, log]);

  const onReset = useCallback(() => {
    if (!wasm) return;
    wasm.resetDemoState();
    setLookupResult(null);
    log("demo state reset");
    refresh();
  }, [wasm, log, refresh]);

  // Guided tour: runs publish → broadcast (waku) → anchor (third party) → lookup
  // with narration in the activity log. Designed for the screen recording — every step
  // is paced so the viewer can follow the visual flow between the three panels.
  const runTour = useCallback(async () => {
    if (!wasm || tourRunning) return;
    setTourRunning(true);
    wasm.resetDemoState();
    setLookupResult(null);
    setStatusLog([]);

    const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
    const sample = new TextEncoder().encode(
      "INTERNAL — Q3 FY26\nSubject: Vendor exposure review\n\nThis document demonstrates the Whistleblower publication pipeline.\nThe content addressed CID below will be discoverable peer-to-peer\nand anchored on-chain without the publisher's coordination.\n",
    );

    setTourStep("Publisher: uploading file to Codex storage");
    log("[1/4] PUBLISHER uploads bytes to Codex…");
    await wait(800);

    const pubResp = wasm.publishFileJson(
      JSON.stringify({
        title: "Vendor exposure review (sample)",
        description: "Sample document for the guided tour.",
        content_type: "text/plain",
        tags: ["sample", "guided-tour"],
        broadcast: true,
      }),
      sample,
    );
    if (!pubResp?.ok) {
      log(`tour aborted: publish failed (${pubResp?.error ?? "unknown"})`);
      setTourRunning(false);
      setTourStep(null);
      return;
    }
    refresh();
    log(`       got CID: ${pubResp.cid.slice(0, 18)}…  (${sample.length} bytes stored)`);
    await wait(1200);

    setTourStep("Broadcaster: envelope published on Waku topic");
    log("[2/4] DELIVERY: envelope broadcast on the Waku topic.");
    log("       Anyone subscribed sees this in real time.");
    await wait(1600);

    setTourStep("Third party: picks up the broadcast and anchors it on-chain");
    log("[3/4] THIRD-PARTY WATCHER anchors the CID on LEZ chronicle-registry.");
    log("       (the publisher did NOT need to be online or hold tokens)");
    const anchorResp = wasm.anchorBatchJson(
      JSON.stringify({
        entries: [{ cid: pubResp.cid, metadata_hash: pubResp.metadata_hash }],
      }),
    );
    if (anchorResp?.ok) {
      log(`       anchored in tx=${anchorResp.tx_hash}`);
    }
    refresh();
    await wait(1400);

    setTourStep("Anyone: query the registry by CID");
    log("[4/4] ANYONE can now query the registry by CID.");
    const lookupResp = wasm.lookupJson(JSON.stringify({ cid: pubResp.cid }));
    setLookupResult(lookupResp);
    setLookupCid(pubResp.cid);
    if (lookupResp?.entry) {
      log(`       lookup → found, anchored at t+${lookupResp.entry.anchor_timestamp}`);
    }
    await wait(1000);

    log("[done] document is now durably published, discoverable, AND on-chain.");
    setTourStep(null);
    setTourRunning(false);
  }, [wasm, tourRunning, log, refresh]);

  const topic = useMemo(() => wasm?.getTopic() ?? "/whistleblower/1/document-index/borsh", [wasm]);

  return (
    <main className="min-h-screen bg-[rgb(12_12_14)] text-[rgb(240_240_240)] font-mono">
      <Hero onRunTour={runTour} tourRunning={tourRunning} canRun={wasm !== null} />

      {tourStep && (
        <div className="bg-lime-500 text-black border-y border-lime-600">
          <div className="max-w-6xl mx-auto px-6 py-3 text-sm font-semibold">
            ▶ Guided tour: {tourStep}
          </div>
        </div>
      )}

      <section className="max-w-6xl mx-auto px-6 py-8 grid grid-cols-1 lg:grid-cols-[1fr_1.4fr] gap-6">
        <PublishPanel
          {...{
            file,
            setFile,
            title,
            setTitle,
            description,
            setDescription,
            contentType,
            setContentType,
            tags,
            setTags,
            broadcast,
            setBroadcast,
            onPublish,
            onAnchorAll,
            onReset,
            wasm,
            wasmError,
          }}
        />

        <div className="space-y-4">
          <LayerPanel
            title="Codex storage"
            subtitle="content-addressed bytes · publisher's responsibility"
            color="violet"
            count={published.length}
          >
            {published.length === 0 ? (
              <Empty text="No uploads yet. Pick a file and click Publish." />
            ) : (
              <ul className="space-y-1.5 text-xs">
                {published.map((r) => (
                  <li key={r.publish_id} className="bg-[rgb(22_22_26)] rounded px-3 py-2 border border-[rgb(40_40_48)]">
                    <div className="flex justify-between gap-2">
                      <span className="truncate">{r.envelope.title}</span>
                      <span className="text-[rgb(130_130_140)] tabular-nums">
                        {r.envelope.size_bytes} B
                      </span>
                    </div>
                    <div className="text-[rgb(130_130_140)] truncate">cid: {r.envelope.cid}</div>
                  </li>
                ))}
              </ul>
            )}
          </LayerPanel>

          <LayerPanel
            title="Waku delivery"
            subtitle={`${topic} · anyone subscribes`}
            color="sky"
            count={deliveryLog.length}
          >
            {deliveryLog.length === 0 ? (
              <Empty text="No envelopes broadcast. Toggle 'Broadcast' on and publish." />
            ) : (
              <ul className="space-y-1.5 text-xs">
                {deliveryLog.map((e) => (
                  <li key={e.cid} className="bg-[rgb(22_22_26)] rounded px-3 py-2 border border-[rgb(40_40_48)]">
                    <div className="flex justify-between gap-2">
                      <span className="truncate">{e.title}</span>
                      <span className="text-[rgb(130_130_140)] tabular-nums">
                        t+{e.timestamp}
                      </span>
                    </div>
                    <div className="text-[rgb(130_130_140)] truncate">cid: {e.cid}</div>
                  </li>
                ))}
              </ul>
            )}
          </LayerPanel>

          <LayerPanel
            title="LEZ chronicle-registry"
            subtitle="on-chain · anchored by any third party"
            color="lime"
            count={registry.length}
          >
            {registry.length === 0 ? (
              <Empty text="No CIDs anchored. Click 'Anchor all broadcast CIDs' to commit." />
            ) : (
              <ul className="space-y-1.5 text-xs">
                {registry.map((e) => (
                  <li key={e.cid} className="bg-[rgb(22_22_26)] rounded px-3 py-2 border border-[rgb(40_40_48)]">
                    <div className="flex justify-between gap-2">
                      <span className="truncate text-lime-400">⚓ anchored</span>
                      <span className="text-[rgb(130_130_140)] tabular-nums">
                        t+{e.anchor_timestamp}
                      </span>
                    </div>
                    <div className="text-[rgb(130_130_140)] truncate">cid: {e.cid}</div>
                  </li>
                ))}
              </ul>
            )}
          </LayerPanel>
        </div>
      </section>

      <section className="max-w-6xl mx-auto px-6 py-6 grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-[rgb(22_22_26)] border border-[rgb(40_40_48)] rounded p-4">
          <h3 className="text-sm uppercase tracking-wider text-[rgb(130_130_140)] mb-3">
            Lookup CID
          </h3>
          <div className="flex gap-2">
            <input
              type="text"
              value={lookupCid}
              onChange={(e) => setLookupCid(e.target.value)}
              placeholder="zDv…"
              className="flex-1 bg-[rgb(12_12_14)] border border-[rgb(40_40_48)] rounded px-3 py-2 text-sm"
            />
            <button
              onClick={onLookup}
              disabled={!wasm || !lookupCid}
              className="bg-[rgb(40_40_48)] hover:bg-[rgb(60_60_72)] disabled:opacity-40 disabled:cursor-not-allowed rounded px-4 text-sm transition"
            >
              Query registry
            </button>
          </div>
          {lookupResult && (
            <pre className="mt-3 text-xs bg-[rgb(12_12_14)] border border-[rgb(40_40_48)] rounded p-3 overflow-auto">
              {JSON.stringify(lookupResult, null, 2)}
            </pre>
          )}
        </div>

        <div className="bg-[rgb(22_22_26)] border border-[rgb(40_40_48)] rounded p-4">
          <h3 className="text-sm uppercase tracking-wider text-[rgb(130_130_140)] mb-3">
            Activity log
          </h3>
          <div className="text-xs space-y-1 max-h-64 overflow-y-auto">
            {statusLog.length === 0 ? (
              <Empty text="Activity will appear here." />
            ) : (
              statusLog.map((line, i) => (
                <div key={i} className="text-[rgb(180_180_190)]">
                  {line}
                </div>
              ))
            )}
          </div>
        </div>
      </section>

      <Footer />
    </main>
  );
}

function Hero({
  onRunTour,
  tourRunning,
  canRun,
}: {
  onRunTour: () => void;
  tourRunning: boolean;
  canRun: boolean;
}) {
  return (
    <header className="border-b border-[rgb(40_40_48)] bg-gradient-to-b from-[rgb(18_18_22)] to-[rgb(12_12_14)]">
      <div className="max-w-6xl mx-auto px-6 py-10">
        <div className="flex items-center gap-3 text-xs uppercase tracking-widest text-[rgb(130_130_140)] mb-3">
          <span className="text-lime-400">●</span>
          <span>LP-0017 · Logos Network λPrize</span>
          <a
            href="https://github.com/warfield2016/whistleblower"
            className="ml-auto hover:text-lime-400 transition"
          >
            github →
          </a>
        </div>
        <h1 className="text-4xl md:text-5xl font-bold tracking-tight mb-3">
          Whistleblower
        </h1>
        <p className="text-[rgb(200_200_210)] max-w-2xl mb-4">
          A censorship-resistant document upload and indexing app for the Logos Basecamp.
          Upload to Codex, broadcast on Waku, anchor on LEZ —{" "}
          <span className="text-lime-400">permissionlessly</span>, by anyone.
        </p>
        <div className="flex flex-wrap items-center gap-3">
          <button
            onClick={onRunTour}
            disabled={!canRun || tourRunning}
            className="bg-lime-500 hover:bg-lime-400 text-black font-semibold disabled:opacity-40 disabled:cursor-not-allowed rounded px-5 py-2 text-sm transition"
          >
            {tourRunning ? "Running tour…" : "▶ Run guided tour"}
          </button>
          <span className="text-xs text-[rgb(130_130_140)]">
            or scroll down to publish your own document
          </span>
        </div>
        <p className="text-xs text-[rgb(130_130_140)] mt-4 max-w-2xl">
          This page runs the same Rust orchestration logic as the production Basecamp module,
          compiled to WebAssembly. Mock storage / delivery / registry are in-process — no Logos
          infrastructure needed to try the UX.
        </p>
      </div>
    </header>
  );
}

function PublishPanel(props: {
  file: File | null;
  setFile: (f: File | null) => void;
  title: string;
  setTitle: (s: string) => void;
  description: string;
  setDescription: (s: string) => void;
  contentType: string;
  setContentType: (s: string) => void;
  tags: string;
  setTags: (s: string) => void;
  broadcast: boolean;
  setBroadcast: (b: boolean) => void;
  onPublish: () => void;
  onAnchorAll: () => void;
  onReset: () => void;
  wasm: WasmApi | null;
  wasmError: string | null;
}) {
  const fileInput = useRef<HTMLInputElement>(null);

  return (
    <div className="bg-[rgb(22_22_26)] border border-[rgb(40_40_48)] rounded p-4 space-y-3 h-fit">
      <h2 className="text-sm uppercase tracking-wider text-[rgb(130_130_140)]">Publish</h2>

      {props.wasmError && (
        <div className="bg-orange-950/30 border border-orange-900 text-orange-300 text-xs rounded p-2">
          WASM error: {props.wasmError}
        </div>
      )}

      <div>
        <label className="text-xs text-[rgb(130_130_140)] block mb-1">File</label>
        <input
          ref={fileInput}
          type="file"
          onChange={(e) => props.setFile(e.target.files?.[0] ?? null)}
          className="text-xs w-full file:mr-3 file:py-1.5 file:px-3 file:rounded file:border-0 file:bg-[rgb(40_40_48)] file:text-[rgb(240_240_240)] file:cursor-pointer hover:file:bg-[rgb(60_60_72)]"
        />
        {props.file && (
          <div className="text-xs text-[rgb(130_130_140)] mt-1">
            {props.file.name} · {props.file.size} bytes
          </div>
        )}
      </div>

      <Field label="Title">
        <input
          type="text"
          value={props.title}
          onChange={(e) => props.setTitle(e.target.value)}
          className="w-full bg-[rgb(12_12_14)] border border-[rgb(40_40_48)] rounded px-2 py-1.5 text-sm"
        />
      </Field>
      <Field label="Description">
        <input
          type="text"
          value={props.description}
          onChange={(e) => props.setDescription(e.target.value)}
          className="w-full bg-[rgb(12_12_14)] border border-[rgb(40_40_48)] rounded px-2 py-1.5 text-sm"
        />
      </Field>
      <div className="grid grid-cols-2 gap-3">
        <Field label="Content type">
          <input
            type="text"
            value={props.contentType}
            onChange={(e) => props.setContentType(e.target.value)}
            className="w-full bg-[rgb(12_12_14)] border border-[rgb(40_40_48)] rounded px-2 py-1.5 text-sm"
          />
        </Field>
        <Field label="Tags (comma-separated)">
          <input
            type="text"
            value={props.tags}
            onChange={(e) => props.setTags(e.target.value)}
            className="w-full bg-[rgb(12_12_14)] border border-[rgb(40_40_48)] rounded px-2 py-1.5 text-sm"
          />
        </Field>
      </div>
      <label className="flex items-center gap-2 text-sm text-[rgb(200_200_210)]">
        <input
          type="checkbox"
          checked={props.broadcast}
          onChange={(e) => props.setBroadcast(e.target.checked)}
        />
        Broadcast on Waku topic after upload
      </label>

      <div className="flex flex-wrap gap-2 pt-2 border-t border-[rgb(40_40_48)]">
        <button
          onClick={props.onPublish}
          disabled={!props.wasm || !props.file || !props.title}
          className="bg-lime-500 hover:bg-lime-400 text-black font-semibold disabled:opacity-40 disabled:cursor-not-allowed rounded px-4 py-2 text-sm transition"
        >
          Publish
        </button>
        <button
          onClick={props.onAnchorAll}
          disabled={!props.wasm}
          className="bg-[rgb(40_40_48)] hover:bg-[rgb(60_60_72)] disabled:opacity-40 disabled:cursor-not-allowed rounded px-4 py-2 text-sm transition"
        >
          Anchor all broadcast CIDs
        </button>
        <button
          onClick={props.onReset}
          disabled={!props.wasm}
          className="ml-auto text-xs text-[rgb(130_130_140)] hover:text-[rgb(240_240_240)] disabled:opacity-40"
        >
          reset demo state
        </button>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="text-xs text-[rgb(130_130_140)] block mb-1">{label}</label>
      {children}
    </div>
  );
}

function LayerPanel({
  title,
  subtitle,
  color,
  count,
  children,
}: {
  title: string;
  subtitle: string;
  color: "violet" | "sky" | "lime";
  count: number;
  children: React.ReactNode;
}) {
  const dot = {
    violet: "bg-violet-400",
    sky: "bg-sky-400",
    lime: "bg-lime-400",
  }[color];
  return (
    <div className="bg-[rgb(22_22_26)] border border-[rgb(40_40_48)] rounded p-4">
      <div className="flex items-start gap-3 mb-3">
        <span className={`mt-1.5 w-2 h-2 rounded-full ${dot}`} />
        <div className="flex-1 min-w-0">
          <h3 className="text-sm">{title}</h3>
          <p className="text-xs text-[rgb(130_130_140)] truncate">{subtitle}</p>
        </div>
        <span className="text-xs text-[rgb(130_130_140)] tabular-nums">{count}</span>
      </div>
      {children}
    </div>
  );
}

function Empty({ text }: { text: string }) {
  return <p className="text-xs text-[rgb(130_130_140)] italic">{text}</p>;
}

function Footer() {
  return (
    <footer className="max-w-6xl mx-auto px-6 py-10 text-xs text-[rgb(130_130_140)] border-t border-[rgb(40_40_48)] mt-6 space-y-2">
      <p>
        This demo runs entirely in your browser. No bytes leave the page. The mocked storage,
        delivery, and registry mimic Codex / Waku / LEZ behaviour with the exact same wire
        format (envelope schema, metadata_hash <code className="text-lime-400">v1:</code>{" "}
        prefix, Borsh-serialized instructions) as the production module at{" "}
        <a
          href="https://github.com/warfield2016/whistleblower"
          className="text-lime-400 hover:underline"
        >
          warfield2016/whistleblower
        </a>
        .
      </p>
      <p>
        To run the actual end-to-end pipeline against a real Logos sequencer, follow the
        instructions in the repo README — the architecture is identical, only the backend
        wires change.
      </p>
    </footer>
  );
}
