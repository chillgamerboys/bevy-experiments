# Phase 4: Turn-Based Movement

**Status:** 📋 Planned
**Goal:** Learn state-dependent logic and grid-based movement

## Overview

Phase 4 implements the core turn-based mechanics. Units will be able to move on the grid, and the game will alternate between player and enemy turns.

## What We'll Build

- `TurnState` enum (PlayerTurn, EnemyTurn)
- `TurnStatus` component tracking unit actions
- Movement system (click adjacent tile → unit moves)
- Valid movement highlighting (green tiles)
- Turn transition logic
- Turn indicator UI

## Key Concepts

- Sub-states and state composition
- Change detection (`Changed<T>`)
- Conditional system execution based on nested states
- Simple pathfinding (adjacent tiles only)
- State transition triggers

## Documentation Coming Soon

This phase will be documented once implementation is complete.
