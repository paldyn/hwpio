import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import * as crypto from "crypto";
import * as vscode from "vscode";
import { HwpEditorProvider } from "./hwp-editor-provider";
import { initWasmHost, HwpDocument } from "./wasm-host";

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

  // rhwp.exportSvg — SVG 내보내기
  context.subscriptions.push(
    vscode.commands.registerCommand("rhwp.exportSvg", async (uri?: vscode.Uri) => {
      const target = resolveUri(uri);
      if (!target) return;
      await cmdExportSvg(target, context.extensionPath);
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

// ── SVG 내보내기 ─────────────────────────────────────────────────

async function cmdExportSvg(uri: vscode.Uri, extensionPath: string): Promise<void> {
  // 출력 폴더 선택 (기본: 파일과 동일 폴더)
  const defaultDir = vscode.Uri.file(path.dirname(uri.fsPath));
  const folders = await vscode.window.showOpenDialog({
    defaultUri: defaultDir,
    canSelectFolders: true,
    canSelectFiles: false,
    canSelectMany: false,
    openLabel: "이 폴더에 SVG 저장",
  });
  if (!folders || folders.length === 0) return;
  const outDir = folders[0].fsPath;

  const baseName = path.basename(uri.fsPath, path.extname(uri.fsPath));

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: `SVG 내보내기: ${path.basename(uri.fsPath)}`,
      cancellable: false,
    },
    async (progress) => {
      try {
        initWasmHost(extensionPath);

        const fileBytes = fs.readFileSync(uri.fsPath);
        const doc: InstanceType<typeof HwpDocument> = new HwpDocument(new Uint8Array(fileBytes));
        doc.setClipEnabled(false);

        const docInfo = JSON.parse(doc.getDocumentInfo());
        const pageCount: number = docInfo.page_count ?? docInfo.pageCount ?? 0;

        if (pageCount === 0) {
          vscode.window.showWarningMessage("페이지가 없는 문서입니다.");
          return;
        }

        for (let i = 0; i < pageCount; i++) {
          progress.report({
            increment: 100 / pageCount,
            message: `${i + 1} / ${pageCount} 페이지`,
          });
          const svg = doc.renderPageSvg(i);
          const outPath = path.join(outDir, `${baseName}_p${i + 1}.svg`);
          fs.writeFileSync(outPath, svg, "utf8");
        }

        doc.free();

        const outDirUri = vscode.Uri.file(outDir);
        const sel = await vscode.window.showInformationMessage(
          `SVG ${pageCount}개 저장 완료 → ${outDir}`,
          "폴더 열기"
        );
        if (sel === "폴더 열기") {
          vscode.commands.executeCommand("revealFileInOS", outDirUri);
        }
      } catch (err: any) {
        vscode.window.showErrorMessage(`SVG 내보내기 실패: ${err.message ?? err}`);
      }
    }
  );
}
