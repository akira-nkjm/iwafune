import type { fixesJa } from "./ja";

/** Row actions of a missing input (enter the value in place, pick a part, go to the page that adds one) (English) */
export const fixesEn: typeof fixesJa = {
  fixes: {
    enterValue: "Enter value",
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
      description: "Type the value; it is saved as typed and clears the missing input.",
      descriptionChoice: "Choose one of the values this setting accepts.",
      choose: "Choose one",
      example: (e: string) => `e.g. ${e}`,
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
      action: {
        part: "Pick a part",
        catalog_item: "Pick a catalog item",
        comm_link: "Pick a link",
        maneuver: "Pick a maneuver",
      },
      title: (path: string) => `Choose a value for ${path}`,
      prompt: {
        part: () => "Choose a part from the design.",
        catalog_item: (kinds: string) => `Choose the item of kind ${kinds} from the catalog.`,
        comm_link: () => "Choose one of the downlinks defined in the comm settings.",
        maneuver: () =>
          "Choose a maneuver that has a dated transfer (departure and arrival dates).",
      },
      label: {
        part: "Part",
        catalog_item: "Catalog item",
        comm_link: "Link",
        maneuver: "Maneuver",
      },
      choose: "Choose one",
      none: {
        part: () => "No part qualifies. Add a part first:",
        catalog_item: (kinds: string) => `The catalog has no item of kind ${kinds}. Add one first:`,
        comm_link: () => "No downlink is defined. Add a downlink first:",
        maneuver: () => "No maneuver has a dated transfer. Add one first:",
      },
      summary: (path: string, chosen: string) => `Set ${path} to ${chosen}`,
      confirm: "Set",
    },
  },
};
