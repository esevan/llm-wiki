import AppKit
import WebKit
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let path = CommandLine.arguments[1]
let output = CommandLine.arguments[2]
let height: CGFloat = CommandLine.arguments.count > 4 ? CGFloat(Double(CommandLine.arguments[4])!) : 900
let width: CGFloat = CommandLine.arguments.count > 3 ? CGFloat(Double(CommandLine.arguments[3])!) : 1440
let frame = NSRect(x: 0, y: 0, width: width, height: height)
let window = NSWindow(contentRect: frame, styleMask: [.borderless], backing: .buffered, defer: false)
let setup = WKWebViewConfiguration()
setup.websiteDataStore = .nonPersistent()
let web = WKWebView(frame: frame, configuration: setup)
window.contentView = web
class Delegate: NSObject, WKNavigationDelegate {
  let output: String
  init(_ output: String) { self.output = output }
  func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
    DispatchQueue.main.asyncAfter(deadline: .now() + 5) {
      webView.evaluateJavaScript("JSON.stringify({checks:window.workspaceDiagnostics,errors:window.fixtureErrors,ready:document.documentElement.dataset.applicationReady,locale:document.documentElement.lang,fonts:document.fonts.status,dialogs:document.querySelectorAll('[role=dialog]').length,overflow:document.documentElement.scrollWidth>innerWidth,modal:document.querySelector('.task-detail')?.getBoundingClientRect().toJSON()})") { result, error in print(result ?? String(describing:error)) }
      let config = WKSnapshotConfiguration()
      config.rect = CGRect(x: 0, y: 0, width: width, height: height)
      webView.takeSnapshot(with: config) { image, error in
        guard let image = image, let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff), let png = rep.representation(using: .png, properties: [:]) else {
          fputs("Snapshot failed: \(String(describing: error))\n", stderr); exit(1)
        }
        do {try png.write(to: URL(fileURLWithPath: self.output)); print(self.output); exit(0)} catch {fputs("Write failed\n",stderr); exit(1)}
      }
    }
  }
}
let delegate = Delegate(output)
web.navigationDelegate = delegate
web.load(URLRequest(url: URL(string:path)!))
DispatchQueue.main.asyncAfter(deadline: .now() + 20) {fputs("Snapshot timeout\n",stderr); exit(1)}
app.run()
