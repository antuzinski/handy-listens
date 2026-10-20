import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import {
  commands,
  type DictionaryFileKind,
  type DictionaryStatus,
} from "@/bindings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";

interface DictionaryFilesProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const DictionaryFiles: React.FC<DictionaryFilesProps> = React.memo(
  ({ descriptionMode = "inline", grouped = false }) => {
    const { t } = useTranslation();
    const [status, setStatus] = useState<DictionaryStatus | null>(null);
    const [sample, setSample] = useState("");
    const [preview, setPreview] = useState<string | null>(null);

    const loadStatus = useCallback(async () => {
      const result = await commands.getDictionaryStatus();
      if (result.status === "ok") {
        setStatus(result.data);
      } else {
        setStatus(null);
        toast.error(result.error);
      }
    }, []);

    // The file is edited outside the app, so refresh when the user returns.
    useEffect(() => {
      loadStatus();
      window.addEventListener("focus", loadStatus);
      return () => window.removeEventListener("focus", loadStatus);
    }, [loadStatus]);

    const openFile = async (kind: DictionaryFileKind) => {
      const result = await commands.openDictionaryFile(kind);
      if (result.status === "error") {
        toast.error(result.error);
      }
    };

    const resetFile = async (kind: DictionaryFileKind) => {
      const confirmed = await ask(
        t("settings.advanced.dictionary.resetConfirm"),
        {
          title: t("settings.advanced.dictionary.resetTitle"),
          kind: "warning",
        },
      );
      if (!confirmed) return;
      const result = await commands.resetDictionaryFile(kind);
      if (result.status === "error") {
        toast.error(result.error);
        return;
      }
      toast.success(t("settings.advanced.dictionary.resetDone"));
      loadStatus();
    };

    const runPreview = async () => {
      if (!sample.trim()) return;
      setPreview(await commands.previewDictionary(sample));
      loadStatus();
    };

    return (
      <>
        <SettingContainer
          title={t("settings.advanced.dictionary.title")}
          description={t("settings.advanced.dictionary.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
          layout="stacked"
        >
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="primary"
              size="md"
              onClick={() => openFile("dictionary")}
            >
              {t("settings.advanced.dictionary.openDictionary")}
            </Button>
            <Button
              variant="secondary"
              size="md"
              onClick={() => openFile("cleanup_prompt")}
            >
              {t("settings.advanced.dictionary.openCleanupPrompt")}
            </Button>
            <Button
              variant="secondary"
              size="md"
              onClick={() => commands.openAppDataDir()}
            >
              {t("settings.advanced.dictionary.openFolder")}
            </Button>
            <Button
              variant="secondary"
              size="md"
              onClick={() => resetFile("dictionary")}
            >
              {t("settings.advanced.dictionary.reset")}
            </Button>
          </div>
          {status && (
            <div className="mt-2 text-xs text-mid-gray">
              {t("settings.advanced.dictionary.status", {
                entries: status.entries,
                conditional: status.conditional,
                inPrompt: status.terms_in_whisper_prompt,
                terms: status.terms_total,
              })}
              {status.errors.length > 0 && (
                <ul className="mt-1 text-red-500">
                  {status.errors.map((error) => (
                    <li key={`${error.line}-${error.message}`}>
                      {t("settings.advanced.dictionary.error", {
                        line: error.line,
                        message: error.message,
                      })}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
          <div className="mt-3 flex items-center gap-2">
            <Input
              type="text"
              className="flex-1"
              value={sample}
              onChange={(e) => setSample(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  runPreview();
                }
              }}
              placeholder={t("settings.advanced.dictionary.previewPlaceholder")}
              variant="compact"
            />
            <Button
              variant="secondary"
              size="md"
              onClick={runPreview}
              disabled={!sample.trim()}
            >
              {t("settings.advanced.dictionary.preview")}
            </Button>
          </div>
          {preview !== null && (
            <div className="mt-2 text-sm">
              <span className="text-mid-gray">
                {t("settings.advanced.dictionary.previewResult")}
              </span>{" "}
              {preview}
            </div>
          )}
        </SettingContainer>
      </>
    );
  },
);
