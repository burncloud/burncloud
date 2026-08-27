import React from 'react';
import {
  BCPageHeader,
  BCCard,
  BCBadge,
  BCStatus
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminSuppliers() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.suppliers.title}
        subtitle={t.admin.suppliers.subtitle}
        conclusion={{
          text: t.admin.suppliers.conclusion,
          type: 'healthy'
        }}
      />

      {/* Directory Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.suppliers.directoryTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.suppliers.directorySubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.suppliers.colSupplier}</th>
                <th className="pb-3 font-semibold">{t.admin.suppliers.colVerification}</th>
                <th className="pb-3 font-semibold">{t.admin.suppliers.colConnectedGpus}</th>
                <th className="pb-3 font-semibold">{t.admin.suppliers.colUptimeSla}</th>
                <th className="pb-3 font-semibold">{t.admin.suppliers.colRevShare}</th>
                <th className="pb-3 font-semibold">{t.admin.suppliers.col30dEarnings}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.suppliers.colStatus}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  name: 'Silicon Bay Data Centers LLC',
                  tier: 'L4 Strategic',
                  gpus: '128x H100 SXM5',
                  uptime: '99.98%',
                  share: '85%',
                  earnings: '$118,400',
                  status: 'Online'
                },
                {
                  name: 'Frankfurt EuroCompute GmbH',
                  tier: 'L3 Professional',
                  gpus: '64x A100 80GB',
                  uptime: '99.94%',
                  share: '80%',
                  earnings: '$52,100',
                  status: 'Online'
                },
                {
                  name: 'Tokyo Cloud Matrix Ltd',
                  tier: 'L3 Professional',
                  gpus: '48x H100 SXM5',
                  uptime: '99.91%',
                  share: '80%',
                  earnings: '$44,800',
                  status: 'Online'
                },
                {
                  name: 'Nordic AI Infra AS',
                  tier: 'L2 Verified',
                  gpus: '24x RTX 4090 Pool',
                  uptime: '99.72%',
                  share: '75%',
                  earnings: '$12,400',
                  status: 'Online'
                }
              ].map((s, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-sans font-bold text-gray-900">{s.name}</td>
                  <td className="py-3.5 font-sans">
                    <BCBadge variant={s.tier.includes('L4') ? 'accent' : 'brand'} size="sm">
                      {s.tier}
                    </BCBadge>
                  </td>
                  <td className="py-3.5 text-gray-700 font-sans">{s.gpus}</td>
                  <td className="py-3.5 text-emerald-700 font-bold">{s.uptime}</td>
                  <td className="py-3.5 font-bold text-gray-900">{s.share}</td>
                  <td className="py-3.5 font-bold text-emerald-700">{s.earnings}</td>
                  <td className="py-3.5 text-right font-sans">
                    <BCStatus status={s.status} />
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
