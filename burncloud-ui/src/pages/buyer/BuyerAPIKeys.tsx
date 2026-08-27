import React, { useState } from 'react';
import {
  Plus,
  Copy,
  Check,
  AlertCircle
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCButton,
  BCBadge,
  BCModal,
  BCInput
} from '@/components/ui';
import { MOCK_BUYER_KEYS, BuyerApiKey } from '@/data/workbenchData';
import { useTranslation } from '@/i18n/I18nContext';

export function BuyerAPIKeys() {
  const { t } = useTranslation();
  const [keys, setKeys] = useState<BuyerApiKey[]>(MOCK_BUYER_KEYS);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isKeyResultModalOpen, setIsKeyResultModalOpen] = useState(false);
  const [newKeyName, setNewKeyName] = useState('');
  const [newKeyRateLimit, setNewKeyRateLimit] = useState(600);
  const [newKeyCap, setNewKeyCap] = useState(500);
  const [createdFullKey, setCreatedFullKey] = useState('');
  const [copied, setCopied] = useState(false);

  const handleCreateKey = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newKeyName.trim()) return;

    const rawRandom = Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
    const fullKey = `demo-bc-${rawRandom}`;
    const newKeyItem: BuyerApiKey = {
      id: `key-${Date.now()}`,
      name: newKeyName,
      keyPrefix: fullKey.slice(0, 11),
      maskedKey: `${fullKey.slice(0, 11)}••••••••••••••••${fullKey.slice(-4)}`,
      created: 'Just now',
      lastUsed: 'Never',
      tier: 'All Tiers',
      rateLimitRpm: Number(newKeyRateLimit),
      monthlySpendCap: Number(newKeyCap),
      spendThisMonth: 0,
      status: 'Active'
    };

    setKeys([newKeyItem, ...keys]);
    setCreatedFullKey(fullKey);
    setIsCreateModalOpen(false);
    setIsKeyResultModalOpen(true);
    setNewKeyName('');
  };

  const handleRevokeKey = (id: string) => {
    setKeys(keys.map(k => k.id === id ? { ...k, status: 'Revoked' } : k));
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.buyer.apiKeys.title}
        subtitle={t.buyer.apiKeys.subtitle}
        conclusion={{
          text: t.buyer.apiKeys.conclusion,
          type: 'healthy'
        }}
        actions={
          <BCButton
            variant="primary"
            size="sm"
            onClick={() => setIsCreateModalOpen(true)}
          >
            <Plus className="w-3.5 h-3.5" />
            <span>{t.buyer.apiKeys.createKeyBtn}</span>
          </BCButton>
        }
      />

      {/* Keys Table */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.buyer.apiKeys.title}</h3>
            <p className="text-xs text-gray-500">{t.buyer.apiKeys.subtitle}</p>
          </div>
          <span className="text-xs font-mono text-gray-400">
            {keys.filter(k => k.status === 'Active').length} {t.common.active}
          </span>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead>
              <tr className="border-b border-gray-100 text-gray-400 font-mono uppercase text-[10px]">
                <th className="pb-3 font-semibold">{t.buyer.apiKeys.colKeyName}</th>
                <th className="pb-3 font-semibold">{t.buyer.apiKeys.colKeySecret}</th>
                <th className="pb-3 font-semibold">{t.buyer.apiKeys.colRateLimit}</th>
                <th className="pb-3 font-semibold">{t.buyer.apiKeys.colSpendCap}</th>
                <th className="pb-3 font-semibold">{t.buyer.apiKeys.colLastUsed}</th>
                <th className="pb-3 font-semibold">{t.common.status}</th>
                <th className="pb-3 font-semibold text-right">{t.buyer.apiKeys.colActions}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {keys.map((k) => (
                <tr key={k.id} className="hover:bg-gray-50/70 transition-colors">
                  <td className="py-3.5 font-medium text-gray-950">
                    <div className="font-semibold text-gray-900">{k.name}</div>
                    <div className="text-[10px] font-mono text-gray-400">{t.buyer.apiKeys.colCreatedAt}: {k.created}</div>
                  </td>
                  <td className="py-3.5 font-mono text-gray-600">{k.maskedKey}</td>
                  <td className="py-3.5 font-mono text-gray-700">{k.rateLimitRpm} req/min</td>
                  <td className="py-3.5 font-mono text-gray-700">
                    <div className="font-semibold text-gray-900">${k.spendThisMonth.toFixed(2)} / ${k.monthlySpendCap}</div>
                    <div className="w-24 bg-gray-100 h-1.5 rounded-full overflow-hidden mt-1">
                      <div
                        className="bg-gray-900 h-full rounded-full"
                        style={{ width: `${Math.min(100, (k.spendThisMonth / k.monthlySpendCap) * 100)}%` }}
                      />
                    </div>
                  </td>
                  <td className="py-3.5 font-mono text-gray-500">{k.lastUsed}</td>
                  <td className="py-3.5">
                    <BCBadge variant={k.status === 'Active' ? 'success' : 'error'} size="sm">
                      {k.status === 'Active' ? t.common.active : t.buyer.apiKeys.revoked}
                    </BCBadge>
                  </td>
                  <td className="py-3.5 text-right">
                    {k.status === 'Active' && (
                      <button
                        onClick={() => handleRevokeKey(k.id)}
                        className="text-xs text-rose-600 hover:text-rose-800 font-medium hover:underline font-mono cursor-pointer"
                      >
                        {t.buyer.apiKeys.revoke}
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </BCCard>

      {/* Modal: Create Key */}
      <BCModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        title={t.buyer.apiKeys.modalTitle}
        subtitle={t.buyer.apiKeys.modalDesc}
      >
        <form onSubmit={handleCreateKey} className="space-y-4">
          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-gray-700">{t.buyer.apiKeys.inputKeyName}</label>
            <BCInput
              type="text"
              placeholder="e.g. Production Cluster (US-East)"
              value={newKeyName}
              onChange={(e) => setNewKeyName(e.target.value)}
              required
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-700">{t.buyer.apiKeys.colRateLimit}</label>
              <BCInput
                type="number"
                value={newKeyRateLimit}
                onChange={(e) => setNewKeyRateLimit(Number(e.target.value))}
                min={10}
                max={5000}
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-700">{t.buyer.apiKeys.inputSpendCap}</label>
              <BCInput
                type="number"
                value={newKeyCap}
                onChange={(e) => setNewKeyCap(Number(e.target.value))}
                min={10}
                max={50000}
              />
            </div>
          </div>

          <div className="pt-3 border-t border-gray-100 flex items-center justify-end gap-2">
            <BCButton
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => setIsCreateModalOpen(false)}
            >
              {t.common.cancel}
            </BCButton>
            <BCButton type="submit" variant="primary" size="sm">
              {t.buyer.apiKeys.createKeyBtn}
            </BCButton>
          </div>
        </form>
      </BCModal>

      {/* Modal: Key Result Display */}
      <BCModal
        isOpen={isKeyResultModalOpen}
        onClose={() => setIsKeyResultModalOpen(false)}
        title={t.buyer.apiKeys.modalTitle}
        subtitle={t.buyer.apiKeys.secretRevealNotice}
      >
        <div className="space-y-4">
          <div className="p-3 bg-amber-50 rounded-xl border border-amber-200 text-xs text-amber-900 flex items-start gap-2">
            <AlertCircle className="w-4 h-4 text-amber-600 flex-shrink-0 mt-0.5" />
            <p>{t.buyer.apiKeys.secretRevealNotice}</p>
          </div>

          <div className="p-3 bg-gray-950 rounded-xl flex items-center justify-between text-xs font-mono text-emerald-400">
            <span className="truncate pr-2">{createdFullKey}</span>
            <BCButton
              variant="secondary"
              size="xs"
              onClick={() => {
                navigator.clipboard.writeText(createdFullKey);
                setCopied(true);
                setTimeout(() => setCopied(false), 2000);
              }}
            >
              {copied ? <Check className="w-3 h-3 text-emerald-600" /> : <Copy className="w-3 h-3" />}
              <span>{copied ? t.common.copied : t.common.copy}</span>
            </BCButton>
          </div>

          <div className="pt-2 flex justify-end">
            <BCButton
              variant="primary"
              size="sm"
              onClick={() => setIsKeyResultModalOpen(false)}
            >
              {t.buyer.apiKeys.copyAndClose}
            </BCButton>
          </div>
        </div>
      </BCModal>
    </div>
  );
}
