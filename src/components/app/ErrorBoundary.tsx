import { isRouteErrorResponse, useRouteError } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/shared/i18n";

export function ErrorBoundary() {
  const { t } = useI18n();
  const error = useRouteError();
  const message = isRouteErrorResponse(error)
    ? `${error.status} ${error.statusText}`
    : error instanceof Error
      ? error.message
      : t("未知路由错误");

  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-6 text-foreground">
      <div className="max-w-xl space-y-4 rounded-lg border bg-card p-6 shadow-sm">
        <div>
          <h1 className="text-xl font-semibold">{t("页面加载失败")}</h1>
          <p className="mt-2 text-sm text-muted-foreground">{message}</p>
        </div>
        <Button onClick={() => window.location.assign("/")}>{t("返回工作台")}</Button>
      </div>
    </div>
  );
}
