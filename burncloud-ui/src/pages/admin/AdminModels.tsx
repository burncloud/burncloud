import React from 'react';
import {
  BCPageHeader,
  BCCard,
  BCBadge,
  BCStatus
} from '@/components/ui';
import { WORKBENCH_MODELS } from '@/data/workbenchData';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminModels() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.models.title}
        subtitle={t.admin.models.subtitle}
        conclusion={{
          text: t.admin.models.conclusion,
          type: 'healthy'
        }}
      />

      {/* Models Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.models.catalogTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.models.catalogSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.models.colModel}</th>
                <th className="pb-3 font-semibold">{t.admin.models.colPassThroughRate}</th>
                <th className="pb-3 font-semibold">{t.admin.models.colContextWindow}</th>
                <th className="pb-3 font-semibold">{t.admin.models.colRoutingTiers}</th>
                <th className="pb-3 font-semibold">{t.admin.models.colTargetP95}</th>
                <th className="pb-3 font-semibold">{t.admin.models.colAvailability}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.models.colStatus}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {WORKBENCH_MODELS.map((m) => (
                <tr key={m.id} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-sans font-bold text-gray-900">{m.name}</td>
                  <td className="py-3.5 text-gray-900 font-bold">
                    ${m.inputPrice1M} / ${m.outputPrice1M}
                  </td>
                  <td className="py-3.5 text-gray-600">{m.contextWindow}</td>
                  <td className="py-3.5">
                    <div className="flex gap-1">
                      {m.supportedTiers.map(tier => (
                        <BCBadge key={tier} variant={tier === 'Performance' ? 'accent' : 'neutral'} size="sm">
                          {tier}
                        </BCBadge>
                      ))}
                    </div>
                  </td>
                  <td className="py-3.5 text-gray-700">{m.p95LatencyMs} ms</td>
                  <td className="py-3.5 text-emerald-700 font-bold">{m.availability}%</td>
                  <td className="py-3.5 text-right font-sans">
                    <BCStatus status={m.status} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>
    </div>
  );
}
