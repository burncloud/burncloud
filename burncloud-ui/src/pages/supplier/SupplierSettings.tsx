import React, { useState } from 'react';
import {
  Key,
  Bell,
  Copy,
  Check,
  Save
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCButton,
  BCInput
} from '@/components/ui';
import { useTranslation } from '@/i18n/I18nContext';

export function SupplierSettings() {
  const { t } = useTranslation();
  const [daemonToken] = useState('demo-node-token');
  const [copied, setCopied] = useState(false);
  const [saved, setSaved] = useState(false);

  const [notificationEmail, setNotificationEmail] = useState('ops@datacenter-infra.io');
  const [webhookUrl, setWebhookUrl] = useState('https://api.datacenter-infra.io/webhooks/burncloud');

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.supplier.settings.title}
        subtitle={t.supplier.settings.subtitle}
        conclusion={{
          text: t.supplier.settings.conclusion,
          type: 'healthy'
        }}
      />

      <div className="grid grid-cols-1 md:grid-cols-12 gap-6">
        {/* Left Col: Daemon Setup & Token */}
        <div className="md:col-span-6 space-y-5">
          <BCCard className="p-6 space-y-4">
            <div className="flex items-center gap-2">
              <Key className="w-4 h-4 text-gray-700" />
              <h3 className="text-sm font-bold text-gray-950">{t.supplier.settings.daemonTokenTitle}</h3>
            </div>
            <p className="text-xs text-gray-500 leading-relaxed">
              {t.supplier.settings.daemonTokenDesc}
            </p>

            <div className="p-3 bg-gray-950 text-gray-100 rounded-xl font-mono text-xs flex items-center justify-between">
              <span className="truncate pr-2">{daemonToken}</span>
              <BCButton
                variant="secondary"
                size="xs"
                onClick={() => {
                  navigator.clipboard.writeText(daemonToken);
                  setCopied(true);
                  setTimeout(() => setCopied(false), 2000);
                }}
              >
                {copied ? <Check className="w-3 h-3 text-emerald-600" /> : <Copy className="w-3 h-3" />}
                <span>{copied ? t.common.copied : t.common.copy}</span>
              </BCButton>
            </div>

            <div className="p-3 bg-gray-50 rounded-xl border border-gray-100 space-y-1 font-mono text-[10px] text-gray-600">
              <div className="text-gray-400">// Installation Command:</div>
              <div className="text-gray-900 select-all">
                curl -sSL https://burncloud.io/install.sh | bash -s -- --token={daemonToken}
              </div>
            </div>
          </BCCard>
        </div>

        {/* Right Col: Notification & Webhooks */}
        <div className="md:col-span-6 space-y-5">
          <BCCard className="p-6 space-y-4">
            <div className="flex items-center gap-2">
              <Bell className="w-4 h-4 text-gray-700" />
              <h3 className="text-sm font-bold text-gray-950">{t.supplier.settings.alertsTitle}</h3>
            </div>

            <form onSubmit={handleSave} className="space-y-4 text-xs">
              <div className="space-y-1.5">
                <label className="font-semibold text-gray-700">{t.supplier.settings.urgentEmail}</label>
                <BCInput
                  type="email"
                  value={notificationEmail}
                  onChange={(e) => setNotificationEmail(e.target.value)}
                  required
                />
              </div>

              <div className="space-y-1.5">
                <label className="font-semibold text-gray-700">{t.supplier.settings.webhookUrl}</label>
                <BCInput
                  type="url"
                  value={webhookUrl}
                  onChange={(e) => setWebhookUrl(e.target.value)}
                  placeholder="https://..."
                />
                <span className="text-[10px] text-gray-400 block font-mono">
                  BurnCloud will POST thermal warnings, PCIe failovers, and payout confirmations.
                </span>
              </div>

              <div className="pt-2 flex justify-end">
                <BCButton type="submit" variant="primary" size="sm">
                  {saved ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Save className="w-3.5 h-3.5" />}
                  <span>{saved ? t.common.saved : t.supplier.settings.savePreferences}</span>
                </BCButton>
              </div>
            </form>
          </BCCard>
        </div>
      </div>
    </div>
  );
}
