import type { overviewJa } from "./ja";

export const overviewEn: typeof overviewJa = {
  overview: {
    title: "Overview",
    subtitle: (name: string, parts: number, modes: number, nets: number, cases: number) =>
      `${name} · ${parts} parts · ${modes} modes · ${nets} nets · ${cases} cases`,
    massMargin: "Mass margin",
    massTotal: (v: string) => `Total ${v}`,
    massDryWet: (dry: string, wet: string) => `Dry ${dry} · wet ${wet}`,
    massBudget: "Mass budget",
    limit: (v: string) => `Limit ${v}`,
    consumption: (v: string) => `Load ${v}`,
    generationLimit: (v: string) => `Expected generation ${v}`,
    allInRange: "All within range",
    nodesToCheck: (n: number) => `${n} nodes need attention`,
    diagnostics: "Diagnostics",
    diagCounts: (e: number, w: number, i: number) => `${e} errors · ${w} warnings · ${i} info`,
    seeAll: "See all",
    noProblems: "No problems",
    budgetError: (kind: string, msg: string) => `Cannot load the ${kind} budget: ${msg}`,
    budgetTitle: (label: string) => `${label} budget`,
    budgetMode: (mode: string) => ` · mode ${mode}`,
    budgetMargin: (v: string) => `Margin ${v}`,
    budgetLimit: (limit: string, margin: string) => `/ limit ${limit} (system margin ${margin})`,
    limitFrom: (id: string) => `Limit from requirement ${id} (edit it there)`,
    budgetDry: "Dry",
    budgetWet: "Wet",
    budgetTank: (id: string) => `Tank ${id}`,
    budgetTankMargin: (m: string) => `Volume margin ${m}`,
    budgetPropellant: "Propellant",
    propellantNone:
      "No propulsion system (maneuvers or propulsion parts), so propellant is not counted",
    propellantMissing: "Propulsion inputs are missing, so propellant is not counted",
    propellantFailed: (msg: string) =>
      `The propulsion analysis failed, so propellant is not counted: ${msg}`,
    budgetOther: "(other)",
    budgetOtherTitle: "Parts without a system or part allocation",
    budgetUncounted: (count: number, names: string) =>
      `${count} part(s) without a value are not in the total (not treated as 0): ${names}`,
    note: (n: string) => `• ${n}`,
  },
};
