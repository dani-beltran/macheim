import { useEffect, useState } from "react";
import { Loader2, Download, CheckCircle, Package } from "lucide-react";
import { listen } from "@tauri-apps/api/event";

interface ProgressEvent {
  stage: string;
  mod_name: string;
  current: number;
  total: number;
  bytes_downloaded: number;
  bytes_total: number | null;
  message: string;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(0)} KB`;
  return `${bytes} B`;
}

export default function ProgressOverlay() {
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const unlisten = listen<ProgressEvent>("mod-progress", (event) => {
      const p = event.payload;
      if (p.stage === "done") {
        // Show done briefly then hide
        setProgress(p);
        setTimeout(() => {
          setVisible(false);
          setProgress(null);
        }, 2000);
      } else {
        setProgress(p);
        setVisible(true);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  if (!visible || !progress) return null;

  const isDone = progress.stage === "done";
  const isDownloading = progress.stage === "downloading";
  const pct = progress.bytes_total
    ? Math.round((progress.bytes_downloaded / progress.bytes_total) * 100)
    : null;
  const overallPct = progress.total > 0
    ? Math.round((progress.current / progress.total) * 100)
    : 0;

  return (
    <div className="fixed inset-0 z-[100] flex items-end justify-center pb-6 pointer-events-none">
      <div className="pointer-events-auto w-full max-w-lg mx-4 rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-bg-sidebar)] shadow-2xl shadow-black/40 overflow-hidden animate-[slideUp_0.2s_ease-out]">
        {/* Overall progress bar */}
        <div className="h-1 bg-[var(--color-bg-input)]">
          <div
            className={`h-full transition-all duration-300 ${isDone ? "bg-[var(--color-success)]" : "bg-[var(--color-accent-primary)]"}`}
            style={{ width: `${isDone ? 100 : overallPct}%` }}
          />
        </div>

        <div className="p-4">
          <div className="flex items-center gap-3">
            {isDone ? (
              <CheckCircle size={20} className="text-[var(--color-success)] shrink-0" />
            ) : isDownloading ? (
              <Download size={20} className="text-[var(--color-accent-primary)] shrink-0 animate-pulse" />
            ) : (
              <Loader2 size={20} className="text-[var(--color-accent-primary)] shrink-0 animate-spin" />
            )}

            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between">
                <p className="text-sm font-medium text-[var(--color-text-primary)] truncate">
                  {progress.message}
                </p>
                {progress.total > 0 && !isDone && (
                  <span className="text-xs text-[var(--color-text-muted)] shrink-0 ml-2">
                    {progress.current}/{progress.total}
                  </span>
                )}
              </div>

              {progress.mod_name && !isDone && (
                <div className="flex items-center gap-2 mt-1">
                  <Package size={12} className="text-[var(--color-text-muted)] shrink-0" />
                  <p className="text-xs text-[var(--color-text-muted)] truncate">
                    {progress.mod_name}
                  </p>
                </div>
              )}

              {isDownloading && progress.bytes_downloaded > 0 && (
                <div className="flex items-center gap-2 mt-1.5">
                  <div className="flex-1 h-1.5 bg-[var(--color-bg-input)] rounded-full overflow-hidden">
                    <div
                      className="h-full bg-[var(--color-accent-amber)] rounded-full transition-all duration-200"
                      style={{ width: `${pct ?? 50}%` }}
                    />
                  </div>
                  <span className="text-[10px] text-[var(--color-text-muted)] shrink-0 tabular-nums">
                    {formatBytes(progress.bytes_downloaded)}
                    {progress.bytes_total ? ` / ${formatBytes(progress.bytes_total)}` : ""}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>

        <style>{`
          @keyframes slideUp {
            from { transform: translateY(100%); opacity: 0; }
            to { transform: translateY(0); opacity: 1; }
          }
        `}</style>
      </div>
    </div>
  );
}
