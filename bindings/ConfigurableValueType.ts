// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.

export type ConfigurableValueType = { type: "String" } & string | null | { type: "Integer", min: number | null, max: number | null, } | { type: "UnsignedInteger", min: number | null, max: number | null, } | { type: "Float", min: number | null, max: number | null, } | { type: "Boolean" } | { type: "Enum", options: Array<string>, };