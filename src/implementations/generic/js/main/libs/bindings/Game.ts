// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { GameType } from "./GameType.ts";
import type { MinecraftVariant } from "./MinecraftVariant.ts";

export type Game = { type: "MinecraftJava", variant: MinecraftVariant, } | { type: "Generic", game_name: GameType, game_display_name: string, };