import { useEffect } from "react";
import { Loader2, Filter } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { useConfigStore } from "@/stores/config.store";
import type { Filter as FilterType } from "@/lib/types";

export function FiltersPage() {
  const { config, loading, load, save, setFilters } = useConfigStore();

  useEffect(() => {
    load();
  }, []);

  useEffect(() => {
    if (config) {
      save()
    }
  }, [config])

  const handleToggleFilter = (filterId: string) => {
    if (!config?.filters) return;

    const updatedFilters = config.filters.map((f) =>
      f.id === filterId ? { ...f, active: !f.active } : f,
    );
    setFilters(updatedFilters);
  };

  if (loading || !config) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-6 h-6 animate-spin" />
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">Фильтры</h1>
        <p className="text-sm text-muted-foreground mt-1">
          WinDivert фильтры для отсечения полезной нагрузки
        </p>
      </div>

      <div className="space-y-3">
        {config.filters?.map((filter: FilterType) => (
          <div
            key={filter.id}
            className="flex items-center justify-between p-4 rounded-lg border bg-card"
          >
            <div className="flex items-center gap-3">
              <Filter className="w-4 h-4 text-muted-foreground" />
              <div>
                <Label
                  htmlFor={filter.id}
                  className="font-medium cursor-pointer"
                >
                  {filter.name}
                </Label>
                <p className="text-xs text-muted-foreground font-mono">
                  {filter.filename}
                </p>
              </div>
            </div>
            <Switch
              id={filter.id}
              checked={filter.active}
              onCheckedChange={() => handleToggleFilter(filter.id)}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
