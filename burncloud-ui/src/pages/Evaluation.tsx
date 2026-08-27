import React from 'react';
import { Card, Badge, Button } from '@/components/ui';
import { Play, Activity } from 'lucide-react';
import { motion } from 'motion/react';

const mockEvals = [
  { model: 'claude-fable-5', scores: { reasoning: 99, coding: 98, chinese: 92, longContext: 94, toolUse: 98, stability: 99, cost: 72, overall: 99 } },
  { model: 'gpt-5.5', scores: { reasoning: 99, coding: 97, chinese: 93, longContext: 96, toolUse: 97, stability: 99, cost: 78, overall: 98 } },
  { model: 'DeepSeek-V4', scores: { reasoning: 98, coding: 96, chinese: 95, longContext: 90, toolUse: 93, stability: 94, cost: 97, overall: 97 } },
  { model: 'grok-4.5', scores: { reasoning: 96, coding: 95, chinese: 91, longContext: 93, toolUse: 96, stability: 96, cost: 78, overall: 96 } },
  { model: 'GLM-5.2', scores: { reasoning: 95, coding: 93, chinese: 96, longContext: 98, toolUse: 94, stability: 95, cost: 84, overall: 95 } },
  { model: 'gemini-3.5-flash', scores: { reasoning: 94, coding: 93, chinese: 90, longContext: 99, toolUse: 95, stability: 97, cost: 98, overall: 95 } },
  { model: 'kimi-k2.7-code', scores: { reasoning: 93, coding: 97, chinese: 92, longContext: 95, toolUse: 94, stability: 95, cost: 94, overall: 94 } },
  { model: 'Qwen/Qwen3.6-35B-A3B', scores: { reasoning: 91, coding: 92, chinese: 96, longContext: 89, toolUse: 88, stability: 94, cost: 95, overall: 93 } },
  { model: 'Seed2.0 Pro', scores: { reasoning: 90, coding: 91, chinese: 94, longContext: 88, toolUse: 89, stability: 93, cost: 94, overall: 92 } },
  { model: 'Llama-4-Maverick', scores: { reasoning: 89, coding: 88, chinese: 87, longContext: 88, toolUse: 88, stability: 96, cost: 100, overall: 91 } },
];

export function Evaluation() {
  return (
    <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 ease-out">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-[26px] font-semibold text-gray-900 tracking-tight">Evaluation</h2>
          <p className="text-gray-500 mt-1.5 text-[14px]">Compare model quality and run regression tests against prompt suites.</p>
        </div>
        <div className="flex items-center gap-3">
          <Button variant="secondary" className="gap-2 text-[13px]"><Activity className="w-4 h-4" /> View Suites</Button>
          <Button className="gap-2 text-[13px]"><Play className="w-4 h-4" /> Run Evaluation</Button>
        </div>
      </div>

      <Card className="overflow-hidden">
        <div className="p-6 border-b border-gray-100 flex items-center justify-between">
          <h3 className="text-[15px] font-medium text-gray-900 tracking-tight">Model Comparison Matrix</h3>
          <span className="text-[12px] text-gray-500">Last updated: 2 hours ago</span>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm text-left">
            <thead className="text-[13px] text-gray-500 bg-transparent border-b border-gray-200/60">
              <tr>
                <th className="px-6 py-4 font-medium">Model</th>
                <th className="px-6 py-4 font-medium text-center">Reasoning</th>
                <th className="px-6 py-4 font-medium text-center">Coding</th>
                <th className="px-6 py-4 font-medium text-center">Chinese</th>
                <th className="px-6 py-4 font-medium text-center">Long Context</th>
                <th className="px-6 py-4 font-medium text-center">Tool Use</th>
                <th className="px-6 py-4 font-medium text-center">Stability</th>
                <th className="px-6 py-4 font-medium text-center">Cost Efficiency</th>
                <th className="px-6 py-4 font-medium text-center">Overall Score</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {mockEvals.map((evalData) => (
                <tr key={evalData.model} className="hover:bg-gray-50/80 transition-colors">
                  <td className="px-6 py-5 font-medium text-[13px] text-gray-900">{evalData.model}</td>
                  {[
                    evalData.scores.reasoning,
                    evalData.scores.coding,
                    evalData.scores.chinese,
                    evalData.scores.longContext,
                    evalData.scores.toolUse,
                    evalData.scores.stability,
                    evalData.scores.cost,
                  ].map((score, idx) => (
                    <td key={idx} className="px-6 py-5 text-center">
                      <div className="flex items-center justify-center">
                        <div className={`w-[34px] h-[34px] rounded-full flex items-center justify-center text-[13px] font-medium tabular-nums ${
                          score >= 95 ? 'bg-green-50 text-green-700 ring-1 ring-inset ring-green-500/20' :
                          score >= 90 ? 'bg-blue-50 text-blue-700 ring-1 ring-inset ring-blue-500/20' :
                          score >= 85 ? 'bg-gray-50 text-gray-700 ring-1 ring-inset ring-gray-500/20' : 'bg-amber-50 text-amber-700 ring-1 ring-inset ring-amber-500/20'
                        }`}>
                          {score}
                        </div>
                      </div>
                    </td>
                  ))}
                  <td className="px-6 py-5 text-center">
                    <Badge variant={evalData.scores.overall >= 93 ? 'success' : 'neutral'} className="text-[13px] px-3 py-1 shadow-sm">
                      {evalData.scores.overall}
                    </Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
