// Run from the task worktree root. Captures only a local nonpersistent WKWebView.
import Cocoa
import WebKit
let args=CommandLine.arguments
let app=NSApplication.shared
app.setActivationPolicy(.accessory)
let width=Double(args[3])!, height=Double(args[4])!
let window=NSWindow(contentRect:NSRect(x:0,y:0,width:width,height:height),styleMask:[.borderless],backing:.buffered,defer:false)
let configuration=WKWebViewConfiguration()
configuration.websiteDataStore = .nonPersistent()
let web=WKWebView(frame:window.contentView!.bounds,configuration:configuration)
window.contentView=web
window.orderFrontRegardless()
class Delegate:NSObject,WKNavigationDelegate {
 var output:String
 init(_ output:String){self.output=output}
 func webView(_ webView:WKWebView,didFinish navigation:WKNavigation!){
  DispatchQueue.main.asyncAfter(deadline:.now()+3.5){
   webView.evaluateJavaScript("(document.querySelector('.vault-reader')||document.querySelector('.reader')).scrollTop="+(CommandLine.arguments.count>5 ? CommandLine.arguments[5]:"0")+(CommandLine.arguments.count>6 ? ";"+CommandLine.arguments[6]:"")){_,_ in
    DispatchQueue.main.asyncAfter(deadline:.now()+0.5){
    let auditPath=URL(fileURLWithPath:FileManager.default.currentDirectoryPath).appendingPathComponent("design/vault-navigation/review-selection-audit.js")
    let audit=try! String(contentsOf:auditPath,encoding:.utf8)
    webView.evaluateJavaScript(audit){result,error in
     guard let value=result as? String, let data=value.data(using:.utf8),let report=try? JSONSerialization.jsonObject(with:data) as? [String:Any] else{print("Selection audit failed",error as Any);exit(1)}
     try? value.write(toFile:self.output.replacingOccurrences(of:".png",with:".selection.json"),atomically:true,encoding:.utf8)
     if let problems=report["problems"] as? [String], !problems.isEmpty { print("Selection audit failed:",problems);exit(1) }
    let config=WKSnapshotConfiguration()
    webView.takeSnapshot(with:config){img,err in
     guard let img=img,let tiff=img.tiffRepresentation,let rep=NSBitmapImageRep(data:tiff),let png=rep.representation(using:.png,properties:[:]) else{print("capture failed",err as Any);exit(1)}
     try! png.write(to:URL(fileURLWithPath:self.output));print(self.output)
     webView.evaluateJavaScript("JSON.stringify({viewport:[innerWidth,innerHeight],bodyWidth:document.body.scrollWidth,browser:document.querySelector('.vault-browser')?.getBoundingClientRect().width,reader:document.querySelector('.vault-reader')?.getBoundingClientRect().width,article:document.querySelector('.vault-note')?.getBoundingClientRect().width,svgCount:document.querySelectorAll('.vault-diagram-scroll svg').length,imageWidth:document.querySelector('.vault-image-button img')?.naturalWidth,title:document.querySelector('.vault-note h1')?.textContent,noteScripts:document.querySelectorAll('.vault-markdown script,.vault-markdown iframe').length,fonts:document.fonts.status})"){value,_ in
      if let value=value as? String {try? value.write(toFile:self.output.replacingOccurrences(of:".png",with:".json"),atomically:true,encoding:.utf8)}
      exit(0)
     }
    }
    }
    }
   }
  }
 }
}
let delegate=Delegate(args[2]);web.navigationDelegate=delegate
web.loadFileURL(URL(fileURLWithPath:args[1]),allowingReadAccessTo:URL(fileURLWithPath:FileManager.default.currentDirectoryPath))
app.run()
