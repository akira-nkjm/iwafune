import { Link } from "react-router";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { PageHeader } from "@/shell/PageHeader";

export default function NotFound() {
  const t = useT();
  return (
    <>
      <PageHeader title={t.shell.pageNotFound} />
      <Empty>
        <Link to="/" className="text-accent-ink underline">
          {t.shell.backToOverview}
        </Link>
      </Empty>
    </>
  );
}
