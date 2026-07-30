import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { ResetButton } from "../ui/ResetButton";
import { useSettings } from "../../hooks/useSettings";
import type { AudioDevice } from "@/bindings";

interface SystemAudioDeviceSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  disabled?: boolean;
}

export const SystemAudioDeviceSelector: React.FC<SystemAudioDeviceSelectorProps> =
  React.memo(
    ({ descriptionMode = "tooltip", grouped = false, disabled = false }) => {
      const { t } = useTranslation();
      const {
        getSetting,
        updateSetting,
        resetSetting,
        isUpdating,
        isLoading,
        outputDevices,
        refreshOutputDevices,
      } = useSettings();

      const selectedSystemAudioDevice =
        getSetting("selected_system_audio_device") === "default"
          ? "Default"
          : getSetting("selected_system_audio_device") || "Default";

      const handleSystemAudioDeviceSelect = async (deviceName: string) => {
        await updateSetting("selected_system_audio_device", deviceName);
      };

      const handleReset = async () => {
        await resetSetting("selected_system_audio_device");
      };

      const systemAudioDeviceOptions = outputDevices.map((device: AudioDevice) => ({
        value: device.name,
        label: device.name,
      }));

      return (
        <SettingContainer
          title={t("settings.sound.systemAudioDevice.title")}
          description={t("settings.sound.systemAudioDevice.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
          disabled={disabled}
        >
          <div className="flex items-center space-x-1">
            <Dropdown
              options={systemAudioDeviceOptions}
              selectedValue={selectedSystemAudioDevice}
              onSelect={handleSystemAudioDeviceSelect}
              placeholder={
                isLoading || outputDevices.length === 0
                  ? t("settings.sound.systemAudioDevice.loading")
                  : t("settings.sound.systemAudioDevice.placeholder")
              }
              disabled={
                disabled ||
                isUpdating("selected_system_audio_device") ||
                isLoading ||
                outputDevices.length === 0
              }
              onRefresh={refreshOutputDevices}
            />
            <ResetButton
              onClick={handleReset}
              disabled={
                disabled || isUpdating("selected_system_audio_device") || isLoading
              }
            />
          </div>
        </SettingContainer>
      );
    },
  );
