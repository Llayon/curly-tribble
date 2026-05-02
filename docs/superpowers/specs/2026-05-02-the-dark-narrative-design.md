# The Dark Narrative: Design Specification

**Date:** 2026-05-02  
**Topic:** Classic Fantasy Idle-Dynasty Settlement Simulator  
**Status:** Draft for Review

---

## 1. Vision Statement
A "living ant farm" experience where players manage a classic fantasy settlement through indirect control, while simultaneously directing a dynastic family through a gritty, choice-driven narrative inspired by *King of Dragon Pass* and *Forbidden Lands*. The game balances the slow satisfaction of idle progression with the high-stakes drama of hereditary rule.

---

## 2. Core Gameplay Pillars

### 2.1. The "Ant Farm" Settlement (Indirect Control)
- **Visuals:** Top-down view (RimWorld-style), clean but atmospheric classic fantasy.
- **Mechanics:** Players define **Zones** (Housing, Stockpiles, Fields) and **Priorities**.
- **Agents:** Commoners (Pawns) follow their own needs (Hunger, Sleep, Morale) and pick tasks based on player-set priorities.
- **Idle Nature:** Resources accumulate while the game is in the background.

### 2.2. The Dynastic Domain (Directive Control)
- **The Inner Circle:** 5-10 named family members per House. The player acts as the **Head of their Family**.
- **Direct Orders:** Family members can be given direct, immediate commands.
- **Succession via Election:** When the current Ruler of the Settlement dies, a new one is elected from the available heads of the Noble Houses.
- **Legacy:** Prestige and Renown earned by your House during the previous reign are spent on permanent boons that persist through all future generations and elections.

### 2.4. Political Landscape: Noble Houses
- **Cooperative Governance:** All houses work together for the prosperity of the city. Rivalry is minimal and focused on earning the most Prestige to win the next election.
- **House Specialization:** Different houses can focus on specific sectors (e.g., House A manages Food, House B manages Defense/Expeditions).
- **AI & Co-op:** AI houses act as supportive partners, contributing to city resources and participating in the election process.

---

## 3. Systems Architecture

### 3.1. Resource & Production Chains
Resources are processed to increase efficiency:
1. **Sustenance:** Grain (Farm) -> Flour (Mill) -> Bread (Bakery).
2. **Construction:** Logs (Woodcutter) -> Planks (Sawmill).
3. **Industry:** Ore (Mine) -> Ingots (Smelter) -> Tools/Weapons (Smithy).
4. **Dynasty:** Wool (Pasture) -> Cloth (Weaver) -> Fine Clothes (Tailor).

### 3.2. Seasonal Cycle
- **Summer:** High food production, faster movement, optimal for expeditions.
- **Winter:** No farming, increased wood/food consumption, survival-focused.
- **Idle Impact:** Players must set up the settlement to survive "off-line" winters.

### 3.3. Expeditions
- **Off-map Progress:** Family members + guards physically leave the map.
- **Progression:** A dedicated UI log shows their journey (e.g., "Entered the Ancient Woods," "Fought a wild bear").
- **Rewards:** Heirlooms (permanent artifacts), rare resources, or new family members.

---

## 4. Technical Implementation (Bevy ECS)

- **Entities:**
    - `Pawn`: Components for `Needs`, `TaskPool`, `Skills`.
    - `FamilyMember`: Specialized `Pawn` with `LegacyStats`, `DirectOrderBuffer`.
    - `Building`: `ProductionChain` component, `WorkerSlot`.
- **Systems:**
    - `BehaviorSystem`: Matches pawns to tasks based on zoning and priorities.
    - `NarrativeEngine`: Triggers events based on timers, resource thresholds, or random seeds.
    - `TimeManager`: Handles catch-up logic for idle time (simulation of missing ticks).
- **Persistence:**
    - Save/Load for current settlement.
    - Global "Family File" for persistent Legacy upgrades.

---

## 5. Success Criteria
1. **Readable Visuals:** A clear "ant farm" where the player can immediately see the health of the settlement.
2. **Impactful Choices:** Events feel heavy and change the course of the family history.
3. **Satisfying Idle Loop:** Returning to the game after an hour provides a sense of progress without constant "game over" restarts.
4. **Emotional Connection:** The player cares about the survival of their specific bloodline.

---

## 6. Self-Review Notes
- **Placeholders:** None. All "Dark" terminology replaced with "Classic Fantasy" or specific mechanics.
- **Consistency:** The transition between "Indirect" (Settlement) and "Direct" (Family) is handled via UI selection/zooming.
- **Scope:** Initial build will focus on the Grain-to-Bread chain and the first Generation of the Family.
