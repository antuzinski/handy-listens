import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { SettingContainer } from "../ui/SettingContainer";
import { Textarea } from "../ui/Textarea";
import { Select } from "../ui/Select";

const PROMPTS: Record<string, string> = {
  ru_punctuation:
    "Я проверил настройки, сохранил файл и продолжил работу. «Всё готово?» — спросил он.",
  ru_en:
    "Я проверил настройки, then saved the file, и продолжил работу. «Всё готово?» — спросил он.",
  en_punctuation:
    "I checked the settings, saved the file, and continued. “Is everything ready?” he asked.",
  ru_instruction:
    "Расшифровывай русскую речь дословно. Ставь естественную пунктуацию. Не переводи и не пересказывай.",
  ru_en_instruction:
    "Расшифровывай речь дословно. Сохраняй русские слова кириллицей, английские термины — латиницей. Не переводи и не пересказывай.",
};

const getPresetPrompt = (preset: string): string => PROMPTS[preset] ?? "";

interface WhisperPromptProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const WhisperPrompt: React.FC<WhisperPromptProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const savedPrompt = getSetting("whisper_prompt") ?? "";
  const savedPreset = getSetting("whisper_prompt_preset") ?? "ru_en";
  const [draft, setDraft] = useState(savedPrompt);
  const [preset, setPreset] = useState(savedPreset);
  const migratedDefault = useRef(false);

  useEffect(() => setDraft(savedPrompt), [savedPrompt]);
  useEffect(() => setPreset(savedPreset), [savedPreset]);

  useEffect(() => {
    if (
      migratedDefault.current ||
      savedPrompt ||
      savedPreset !== "ru_en" ||
      isUpdating("whisper_prompt")
    ) {
      return;
    }
    migratedDefault.current = true;
    const initialPrompt = getPresetPrompt("ru_en");
    setDraft(initialPrompt);
    void updateSetting("whisper_prompt", initialPrompt);
  }, [isUpdating, savedPreset, savedPrompt, updateSetting]);

  const presetOptions = [
    { value: "ru_punctuation", label: t("settings.advanced.whisperPrompt.presets.ru") },
    { value: "ru_en", label: t("settings.advanced.whisperPrompt.presets.ruEn") },
    { value: "en_punctuation", label: t("settings.advanced.whisperPrompt.presets.en") },
    { value: "ru_instruction", label: t("settings.advanced.whisperPrompt.presets.ruInstruction") },
    { value: "ru_en_instruction", label: t("settings.advanced.whisperPrompt.presets.ruEnInstruction") },
    { value: "custom", label: t("settings.advanced.whisperPrompt.presets.custom") },
    { value: "none", label: t("settings.advanced.whisperPrompt.presets.none") },
  ];

  const handlePresetChange = (value: string | null) => {
    if (!value) return;
    setPreset(value);
    void updateSetting("whisper_prompt_preset", value);
    const nextPrompt = value === "custom" ? savedPrompt : getPresetPrompt(value);
    setDraft(nextPrompt);
    void updateSetting("whisper_prompt", nextPrompt);
  };

  return (
    <SettingContainer
      title={t("settings.advanced.whisperPrompt.title")}
      description={t("settings.advanced.whisperPrompt.description")}
      descriptionMode="inline"
      grouped={grouped}
      layout="stacked"
    >
      <div className="space-y-3">
        <label className="block text-xs font-medium text-mid-gray" htmlFor="whisper-prompt-preset">
          {t("settings.advanced.whisperPrompt.presetLabel")}
        </label>
        <Select
          value={preset}
          options={presetOptions}
          isClearable={false}
          onChange={handlePresetChange}
          disabled={isUpdating("whisper_prompt_preset")}
          className="w-full"
        />
        <label className="block text-xs font-medium text-mid-gray" htmlFor="whisper-prompt-text">
          {t("settings.advanced.whisperPrompt.textLabel")}
        </label>
        <Textarea
          id="whisper-prompt-text"
          className="whisper-prompt-editor w-full"
          rows={4}
          maxLength={1000}
          value={preset === "custom" || preset === "none" ? draft : getPresetPrompt(preset)}
          onChange={(event) => {
            setDraft(event.target.value);
            if (preset !== "custom") {
              setPreset("custom");
              void updateSetting("whisper_prompt_preset", "custom");
            }
          }}
          onBlur={() => {
            if (draft !== savedPrompt) {
              void updateSetting("whisper_prompt", draft.trim());
            }
          }}
          placeholder={t("settings.advanced.whisperPrompt.placeholder")}
          disabled={isUpdating("whisper_prompt")}
          aria-label={t("settings.advanced.whisperPrompt.textLabel")}
        />
      </div>
    </SettingContainer>
  );
};
