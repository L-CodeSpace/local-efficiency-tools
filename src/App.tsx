import { RouterProvider } from "react-router-dom";
import { router } from "@/router";
import { I18nProvider } from "@/shared/i18n";

export default function App() {
  return (
    <I18nProvider>
      <RouterProvider router={router} />
    </I18nProvider>
  );
}
