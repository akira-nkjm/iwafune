import { index, layout, type RouteConfig, route } from "@react-router/dev/routes";

/** 画面の URL（サイドバーの並びは lib/nav.ts） */
export default [
  layout("routes/layout.tsx", [
    index("routes/overview.tsx"),
    route("design/components/:id?", "routes/components.tsx"),
    route("design/catalog/:id?", "routes/catalog.tsx"),
    route("design/diagram", "routes/diagram.tsx"),
    route("design/modes", "routes/modes.tsx"),
    route("design/defs/:tab?", "routes/defs.tsx"),
    route("systems/:system/:tab?", "routes/system.tsx"),
    route("mission/orbit", "routes/mission-orbit.tsx"),
    route("mission/cases", "routes/mission-cases.tsx"),
    route("project/diagnostics", "routes/diagnostics.tsx"),
    route("project/settings", "routes/settings.tsx"),
    route("*", "routes/not-found.tsx"),
  ]),
] satisfies RouteConfig;
