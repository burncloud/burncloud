import React from 'react';
import {
  BCPageHeader,
  BCCard,
  BCBadge,
  BCStatus
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminCustomers() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.customers.title}
        subtitle={t.admin.customers.subtitle}
        conclusion={{
          text: t.admin.customers.conclusion,
          type: 'healthy'
        }}
      />

      {/* Tenants Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.customers.accountsTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.customers.accountsSubtitle}</p>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs font-mono">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.admin.customers.colTenantOrg}</th>
                <th className="pb-3 font-semibold">{t.admin.customers.colContactEmail}</th>
                <th className="pb-3 font-semibold">{t.admin.customers.colActiveApiKeys}</th>
                <th className="pb-3 font-semibold">{t.admin.customers.col30dSpend}</th>
                <th className="pb-3 font-semibold">{t.admin.customers.colEscrowBalance}</th>
                <th className="pb-3 font-semibold">{t.admin.customers.colCreditRisk}</th>
                <th className="pb-3 font-semibold text-right">{t.admin.customers.colStatus}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {[
                {
                  name: 'HyperScale AI Labs',
                  email: 'infra@hyperscale.ai',
                  keys: 8,
                  spend: '$48,200',
                  balance: '$18,400.00',
                  risk: 'Low Risk',
                  status: 'Active'
                },
                {
                  name: 'Apex Robotics Co.',
                  email: 'ops@apexrobotics.com',
                  keys: 4,
                  spend: '$24,100',
                  balance: '$6,200.00',
                  risk: 'Low Risk',
                  status: 'Active'
                },
                {
                  name: 'FinTech Intelligence Inc.',
                  email: 'security@fintech-intel.io',
                  keys: 12,
                  spend: '$19,800',
                  balance: '$12,850.00',
                  risk: 'Low Risk',
                  status: 'Active'
                },
                {
                  name: 'AutoAgent Studio',
                  email: 'dev@autoagent.ai',
                  keys: 2,
                  spend: '$4,120',
                  balance: '$128.50',
                  risk: 'Low Balance',
                  status: 'Active'
                }
              ].map((c, idx) => (
                <tr key={idx} className="hover:bg-gray-50/70">
                  <td className="py-3.5 font-sans font-bold text-gray-900">{c.name}</td>
                  <td className="py-3.5 text-gray-500 font-sans">{c.email}</td>
                  <td className="py-3.5 text-gray-700">{c.keys} keys</td>
                  <td className="py-3.5 font-bold text-gray-950">{c.spend}</td>
                  <td className="py-3.5 font-bold text-emerald-700">{c.balance}</td>
                  <td className="py-3.5 font-sans">
                    <BCBadge variant={c.risk === 'Low Risk' ? 'success' : 'warning'} size="sm">
                      {c.risk}
                    </BCBadge>
                  </td>
                  <td className="py-3.5 text-right font-sans">
                    <BCStatus status={c.status} />
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
