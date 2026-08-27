import React, { useState } from 'react';
import {
  ShieldAlert
} from 'lucide-react';
import {
  BCPageHeader,
  BCCard,
  BCButton,
  BCBadge
} from '@/components/ui';
import { MOCK_AUTOPILOT_LOGS } from '@/data/workbenchData';
import { useTranslation } from '@/i18n/I18nContext';

export function AdminOperations() {
  const { t } = useTranslation();
  const [circuitBreakerActive, setCircuitBreakerActive] = useState(false);

  return (
    <div className="space-y-6">
      {/* Header */}
      <BCPageHeader
        title={t.admin.operations.title}
        subtitle={t.admin.operations.subtitle}
        conclusion={{
          text: t.admin.operations.conclusion,
          type: 'healthy'
        }}
        actions={
          <BCButton
            variant={circuitBreakerActive ? 'danger' : 'secondary'}
            size="sm"
            onClick={() => setCircuitBreakerActive(!circuitBreakerActive)}
          >
            <ShieldAlert className="w-3.5 h-3.5" />
            <span>{t.admin.operations.btnCircuitBreaker}: {circuitBreakerActive ? 'ARMED' : 'STANDBY'}</span>
          </BCButton>
        }
      />

      {/* Incident Log Feed */}
      <BCCard className="p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-base font-bold text-gray-950">{t.admin.operations.streamTitle}</h3>
            <p className="text-xs text-gray-500">{t.admin.operations.streamSubtitle}</p>
          </div>
        </div>

        <div className="space-y-3">
          {MOCK_AUTOPILOT_LOGS.map((log) => (
            <div key={log.id} className="p-4 rounded-xl bg-gray-50 border border-gray-100 flex flex-col sm:flex-row sm:items-start justify-between gap-4 text-xs font-mono">
              <div className="space-y-1.5 font-sans">
                <div className="flex items-center gap-2">
                  <BCBadge
                    variant={log.level === 'Action' ? 'brand' : log.level === 'Optimization' ? 'success' : 'warning'}
                    size="sm"
                  >
                    {log.category}
                  </BCBadge>
                  <span className="font-bold text-gray-900">{log.title}</span>
                </div>
                <p className="text-gray-600 text-xs leading-relaxed">{log.description}</p>
                <div className="pt-1 font-mono text-xs flex items-center gap-3">
                  <span className="text-emerald-700 font-bold">Outcome: {log.impact}</span>
                  <span className="text-gray-400">•</span>
                  <span className="text-gray-500 font-medium">Executed: {log.actionTaken}</span>
                </div>
              </div>
              <span className="text-[10px] text-gray-400 whitespace-nowrap">{log.time}</span>
            </div>
          ))}
        </div>
      </BCCard>
    </div>
  );
}
