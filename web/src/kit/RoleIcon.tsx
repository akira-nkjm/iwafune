import {
  Antenna,
  Battery,
  Box,
  Camera,
  Cpu,
  Disc3,
  Flame,
  type LucideIcon,
  Magnet,
  MapPin,
  Package,
  PlugZap,
  RadioTower,
  Rocket,
  Satellite,
  Square,
  Star,
  Sun,
  SunMedium,
  Thermometer,
} from "lucide-react";

/**
 * 部品の役割（role。種別定義の role）→ アイコン。コンポーネントの一覧でカードの左に出す。
 * 役割の名前は設計の語彙（docs/model.md）で、知らない役割は箱のアイコン。
 */
const ICONS: Record<string, LucideIcon> = {
  battery: Battery,
  solar_panel: Sun,
  pcdu: PlugZap,
  obc: Cpu,
  reaction_wheel: Disc3,
  magnetorquer: Magnet,
  star_tracker: Star,
  sun_sensor: SunMedium,
  gyro: Satellite,
  gps: MapPin,
  camera: Camera,
  transceiver: RadioTower,
  antenna: Antenna,
  heater: Flame,
  thermistor: Thermometer,
  thruster: Rocket,
  structure: Box,
  panel: Square,
};

/** 役割のアイコン（役割が無いか知らないものは箱） */
export function RoleIcon({
  role,
  size = 18,
  className,
}: {
  role: string | null | undefined;
  size?: number;
  className?: string;
}) {
  const Icon = (role && ICONS[role]) || Package;
  return <Icon size={size} strokeWidth={1.8} aria-hidden className={className} />;
}
