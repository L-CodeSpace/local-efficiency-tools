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
  return (
    <Dialog open={!!plan} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>确认执行计划</DialogTitle>
          <DialogDescription>{plan?.summary}</DialogDescription>
        </DialogHeader>
        <div className="rounded-md border bg-muted/30 p-3 text-xs">
          <div>风险：{plan?.risk}</div>
          <div className="mt-1 font-mono">Token: {plan?.confirmationToken}</div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onCancel}>
            取消
          </Button>
          <Button onClick={onConfirm} disabled={busy}>
            <ShieldCheck className="h-4 w-4" />
            确认执行
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
