// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { ConfigurableValue } from "./ConfigurableValue.ts";
import type { ConfigurableValueType } from "./ConfigurableValueType.ts";

export interface SettingManifest {
  setting_id: string;
  name: string;
  description: string;
  value: ConfigurableValue | null;
  value_type: ConfigurableValueType;
  default_value: ConfigurableValue | null;
  is_secret: boolean;
  is_required: boolean;
  is_mutable: boolean;
}
