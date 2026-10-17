/**
 * Categories settings tab (TASK-DSN-053): the shared category management
 * panel embedded as an always-open command-center page. Mounted under
 * Settings → Magasin → Rayons & Catégories.
 */

import { type ReactElement } from "react";
import { CategoryManagePanel } from "@/features/catalog/CategoryManageModal";

interface CategorySettingsTabProps {
  sessionToken: string;
  onChanged: () => void;
}

export function CategorySettingsTab({
  sessionToken,
  onChanged,
}: CategorySettingsTabProps): ReactElement {
  return (
    <div className="flex min-h-0 flex-1 flex-col p-4">
      <CategoryManagePanel token={sessionToken} onChanged={onChanged} />
    </div>
  );
}
