import * as vscode from "vscode";
import { HwpEditorProvider } from "./hwp-editor-provider";

export function activate(context: vscode.ExtensionContext) {
  const { provider, disposable } = HwpEditorProvider.register(context);
  context.subscriptions.push(disposable);

  // rhwp.print — 해당 파일의 webview에 인쇄 요청
  context.subscriptions.push(
    vscode.commands.registerCommand("rhwp.print", async (uri?: vscode.Uri) => {
      const target = resolveUri(uri);
      if (!target) return;
      await provider.sendPrint(target);
    })
  );

  // rhwp.exportSvg — SVG 내보내기 (3단계 구현 예정)
  context.subscriptions.push(
    vscode.commands.registerCommand("rhwp.exportSvg", (_uri?: vscode.Uri) => {
      vscode.window.showInformationMessage("SVG 내보내기 기능을 준비 중입니다.");
    })
  );

  // rhwp.debugOverlay — 디버그 오버레이 (4단계 구현 예정)
  context.subscriptions.push(
    vscode.commands.registerCommand("rhwp.debugOverlay", (_uri?: vscode.Uri) => {
      vscode.window.showInformationMessage("디버그 오버레이 기능을 준비 중입니다.");
    })
  );

  // rhwp.dumpParagraph — 문단 덤프 (4단계 구현 예정)
  context.subscriptions.push(
    vscode.commands.registerCommand("rhwp.dumpParagraph", (_uri?: vscode.Uri) => {
      vscode.window.showInformationMessage("문단 덤프 기능을 준비 중입니다.");
    })
  );
}

export function deactivate() {}

/** 컨텍스트 메뉴에서 전달된 uri, 또는 현재 활성 편집기의 uri를 반환 */
function resolveUri(uri?: vscode.Uri): vscode.Uri | undefined {
  if (uri) return uri;
  const activeUri = vscode.window.activeTextEditor?.document.uri;
  if (activeUri) return activeUri;
  return undefined;
}
