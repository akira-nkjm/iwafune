import type { fixesJa } from "./ja";

/** Row actions of a missing input (enter the value in place, pick a part, go to the page that adds one) (English) */
export const fixesEn: typeof fixesJa = {
  fixes: {
    enterValue: "Enter value",
    pickPart: "Pick a part",
    addPart: "Add a part",
    assignPart: "Assign an existing part",
    setOnPart: "Set on a part",
    addItem: {
      mode: "Add a mode",
      maneuver: "Add a maneuver",
      ground_station: "Add a ground station",
      comm_link: "Add a comm link",
      case: "Add an analysis case",
    },
    value: {
      title: (path: string) => `Enter a value for ${path}`,
      description: "Saves the value as typed and clears the missing input.",
      label: "Value",
      dimension: (d: string) => `Dimension: ${d}`,
      scope: "Write to",
      part: "Override on this part",
      catalog: (item: string) => `Catalog item ${item}`,
      scopeHint:
        "This part uses a catalog item. Writing to the item applies to every part that uses it.",
      summary: (path: string) => `Set ${path}`,
      confirm: "Save",
    },
    pick: {
      title: (path: string) => `Pick a part for ${path}`,
      description: "Choose among the parts in the design.",
      part: "Part",
      choose: "Choose one",
      none: "No part qualifies. Add a part first.",
      summary: (path: string, part: string) => `Set ${path} to ${part}`,
      confirm: "Set",
    },
  },
};
