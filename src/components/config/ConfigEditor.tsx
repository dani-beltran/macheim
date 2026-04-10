import { useState, useEffect, useCallback } from "react";
import {
  FileText,
  Save,
  RotateCcw,
  Loader2,
  ChevronRight,
  Settings,
} from "lucide-react";
import { useAppStore } from "../../store/appStore";
import { getConfigFiles, getConfig, saveConfig } from "../../lib/tauri";
import type { ConfigFile, ConfigEntry, ConfigSection } from "../../lib/types";

export default function ConfigEditor() {
  const addToast = useAppStore((s) => s.addToast);

  const [configFiles, setConfigFiles] = useState<ConfigFile[]>([]);
  const [selectedFile, setSelectedFile] = useState<ConfigFile | null>(null);
  const [isLoadingFiles, setIsLoadingFiles] = useState(true);
  const [isLoadingConfig, setIsLoadingConfig] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [editedEntries, setEditedEntries] = useState<Map<string, string>>(
    new Map()
  );

  // Load config file list
  useEffect(() => {
    async function load() {
      setIsLoadingFiles(true);
      try {
        const files = await getConfigFiles();
        setConfigFiles(files);
      } catch {
        // Backend may not support this yet
        setConfigFiles([]);
      } finally {
        setIsLoadingFiles(false);
      }
    }
    load();
  }, []);

  const handleSelectFile = useCallback(
    async (file: ConfigFile) => {
      setIsLoadingConfig(true);
      setEditedEntries(new Map());
      try {
        const detail = await getConfig(file.filename);
        setSelectedFile(detail);
      } catch {
        // Fallback: show what we already have
        setSelectedFile(file);
      } finally {
        setIsLoadingConfig(false);
      }
    },
    []
  );

  const handleEntryChange = (
    sectionName: string,
    key: string,
    value: string
  ) => {
    const entryKey = `${sectionName}::${key}`;
    setEditedEntries((prev) => {
      const next = new Map(prev);
      next.set(entryKey, value);
      return next;
    });
  };

  const getEntryValue = (
    sectionName: string,
    entry: ConfigEntry
  ): string => {
    const key = `${sectionName}::${entry.key}`;
    return editedEntries.get(key) ?? entry.value;
  };

  const handleSave = async () => {
    if (!selectedFile) return;

    setIsSaving(true);
    try {
      const allEntries: ConfigEntry[] = [];
      for (const section of selectedFile.sections) {
        for (const entry of section.entries) {
          const key = `${section.name}::${entry.key}`;
          const newVal = editedEntries.get(key);
          allEntries.push({
            ...entry,
            value: newVal ?? entry.value,
          });
        }
      }
      await saveConfig(selectedFile.filename, allEntries);
      setEditedEntries(new Map());
      addToast({ type: "success", message: "Config saved." });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to save config: ${err}`,
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleReset = () => {
    setEditedEntries(new Map());
  };

  const hasChanges = editedEntries.size > 0;

  const renderInput = (
    section: ConfigSection,
    entry: ConfigEntry
  ) => {
    const value = getEntryValue(section.name, entry);

    // Boolean toggle
    if (
      entry.setting_type.toLowerCase() === "boolean" ||
      entry.setting_type.toLowerCase() === "bool"
    ) {
      const isTrue = value.toLowerCase() === "true";
      return (
        <button
          onClick={() =>
            handleEntryChange(section.name, entry.key, isTrue ? "false" : "true")
          }
          className={`relative w-10 h-5.5 rounded-full shrink-0 transition-colors cursor-pointer
            ${isTrue ? "bg-[var(--color-accent-primary)]" : "bg-[var(--color-border-default)]"}
          `}
        >
          <div
            className={`absolute top-0.5 w-4.5 h-4.5 rounded-full bg-white shadow transition-transform
              ${isTrue ? "translate-x-5" : "translate-x-0.5"}
            `}
          />
        </button>
      );
    }

    // Dropdown for acceptable values
    if (entry.acceptable_values && entry.acceptable_values.length > 0) {
      return (
        <select
          value={value}
          onChange={(e) =>
            handleEntryChange(section.name, entry.key, e.target.value)
          }
          className="px-2.5 py-1.5 rounded-md text-sm bg-[var(--color-bg-input)] border border-[var(--color-border-default)]
            text-[var(--color-text-primary)] focus:outline-none focus:border-[var(--color-accent-primary)]"
        >
          {entry.acceptable_values.map((av) => (
            <option key={av} value={av}>
              {av}
            </option>
          ))}
        </select>
      );
    }

    // Number input for int/float with range
    if (
      entry.setting_type.toLowerCase().includes("int") ||
      entry.setting_type.toLowerCase().includes("float") ||
      entry.setting_type.toLowerCase().includes("single") ||
      entry.setting_type.toLowerCase().includes("double")
    ) {
      return (
        <div className="flex items-center gap-2">
          <input
            type="number"
            value={value}
            onChange={(e) =>
              handleEntryChange(section.name, entry.key, e.target.value)
            }
            min={entry.acceptable_range?.[0]}
            max={entry.acceptable_range?.[1]}
            className="w-28 px-2.5 py-1.5 rounded-md text-sm bg-[var(--color-bg-input)] border border-[var(--color-border-default)]
              text-[var(--color-text-primary)] focus:outline-none focus:border-[var(--color-accent-primary)]"
          />
          {entry.acceptable_range && (
            <span className="text-xs text-[var(--color-text-muted)]">
              [{entry.acceptable_range[0]} - {entry.acceptable_range[1]}]
            </span>
          )}
        </div>
      );
    }

    // Default: text input
    return (
      <input
        type="text"
        value={value}
        onChange={(e) =>
          handleEntryChange(section.name, entry.key, e.target.value)
        }
        className="w-60 px-2.5 py-1.5 rounded-md text-sm bg-[var(--color-bg-input)] border border-[var(--color-border-default)]
          text-[var(--color-text-primary)] focus:outline-none focus:border-[var(--color-accent-primary)]"
      />
    );
  };

  return (
    <div className="flex gap-5 h-[calc(100vh-8rem)]">
      {/* File list */}
      <div className="w-64 shrink-0 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-bg-card)] overflow-hidden flex flex-col">
        <div className="px-4 py-3 border-b border-[var(--color-border-subtle)]">
          <h3 className="text-sm font-semibold text-[var(--color-text-primary)]">
            Config Files
          </h3>
        </div>
        <div className="flex-1 overflow-y-auto">
          {isLoadingFiles ? (
            <div className="flex items-center justify-center py-8">
              <Loader2
                size={20}
                className="animate-spin text-[var(--color-text-muted)]"
              />
            </div>
          ) : configFiles.length === 0 ? (
            <div className="px-4 py-8 text-center">
              <FileText
                size={28}
                className="mx-auto text-[var(--color-text-muted)] mb-2"
              />
              <p className="text-xs text-[var(--color-text-muted)]">
                No config files found. Install some mods first.
              </p>
            </div>
          ) : (
            configFiles.map((file) => (
              <button
                key={file.filename}
                onClick={() => handleSelectFile(file)}
                className={`w-full flex items-center gap-2 px-4 py-2.5 text-left text-sm transition-colors cursor-pointer
                  ${
                    selectedFile?.filename === file.filename
                      ? "bg-[var(--color-accent-primary)]/10 text-[var(--color-accent-primary)] border-r-2 border-[var(--color-accent-primary)]"
                      : "text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-card-hover)] hover:text-[var(--color-text-primary)]"
                  }
                `}
              >
                <FileText size={14} className="shrink-0" />
                <span className="truncate">{file.mod_name || file.filename}</span>
                <ChevronRight size={12} className="ml-auto shrink-0 opacity-40" />
              </button>
            ))
          )}
        </div>
      </div>

      {/* Editor */}
      <div className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-bg-card)] overflow-hidden flex flex-col">
        {!selectedFile ? (
          <div className="flex-1 flex flex-col items-center justify-center text-center">
            <Settings
              size={40}
              className="text-[var(--color-text-muted)] mb-3"
            />
            <h3 className="text-base font-semibold text-[var(--color-text-secondary)] mb-1">
              Select a config file
            </h3>
            <p className="text-sm text-[var(--color-text-muted)]">
              Choose a config file from the left to edit its settings.
            </p>
          </div>
        ) : (
          <>
            {/* Editor header */}
            <div className="flex items-center justify-between px-5 py-3 border-b border-[var(--color-border-subtle)]">
              <div>
                <h3 className="text-sm font-semibold text-[var(--color-text-primary)]">
                  {selectedFile.mod_name || selectedFile.filename}
                </h3>
                <p className="text-xs text-[var(--color-text-muted)]">
                  {selectedFile.filename}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleReset}
                  disabled={!hasChanges}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium
                    border border-[var(--color-border-default)] text-[var(--color-text-secondary)]
                    hover:bg-[var(--color-bg-elevated)] disabled:opacity-40 transition-colors cursor-pointer"
                >
                  <RotateCcw size={13} />
                  Reset
                </button>
                <button
                  onClick={handleSave}
                  disabled={!hasChanges || isSaving}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium
                    bg-[var(--color-accent-primary)] text-white
                    hover:bg-[var(--color-accent-primary-hover)] disabled:opacity-50 transition-colors cursor-pointer"
                >
                  {isSaving ? (
                    <Loader2 size={13} className="animate-spin" />
                  ) : (
                    <Save size={13} />
                  )}
                  Save
                </button>
              </div>
            </div>

            {/* Config entries */}
            <div className="flex-1 overflow-y-auto px-5 py-4 space-y-6">
              {isLoadingConfig ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2
                    size={20}
                    className="animate-spin text-[var(--color-text-muted)]"
                  />
                </div>
              ) : (
                selectedFile.sections.map((section) => (
                  <div key={section.name}>
                    <h4 className="text-xs font-bold text-[var(--color-accent-amber)] uppercase tracking-wider mb-3 pb-2 border-b border-[var(--color-border-subtle)]">
                      {section.name}
                    </h4>
                    <div className="space-y-4">
                      {section.entries.map((entry) => (
                        <div
                          key={`${section.name}-${entry.key}`}
                          className="flex items-start justify-between gap-4"
                        >
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                              <span className="text-sm font-medium text-[var(--color-text-primary)]">
                                {entry.key}
                              </span>
                              {entry.default_value && (
                                <span className="text-[10px] text-[var(--color-text-muted)] font-mono">
                                  default: {entry.default_value}
                                </span>
                              )}
                            </div>
                            {entry.description && (
                              <p className="text-xs text-[var(--color-text-muted)] mt-0.5 leading-relaxed">
                                {entry.description}
                              </p>
                            )}
                          </div>
                          <div className="shrink-0">
                            {renderInput(section, entry)}
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                ))
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
