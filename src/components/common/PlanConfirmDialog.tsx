import { ShieldCheck } from "lucide-react";
import { type FileOperationPlan } from "@/api_tauri";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useI18n } from "@/shared/i18n";

export function PlanConfirmDialog({
  plan,
  busy,
  onCancel,
  onConfirm,
}: {
  plan: FileOperationPlan | null;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();

  return (
    <Dialog open={!!plan} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("确认执行计划")}</DialogTitle>
          <DialogDescription>{plan?.summary}</DialogDescription>
        </DialogHeader>
        <div className="rounded-md border bg-muted/30 p-3 text-xs">
          <div>{t("风险")}：{plan?.risk}</div>
          <div className="mt-1 font-mono">Token: {plan?.confirmationToken}</div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onCancel}>
            {t("取消")}
          </Button>
          <Button onClick={onConfirm} disabled={busy}>
            <ShieldCheck className="h-4 w-4" />
            {t("确认执行")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
