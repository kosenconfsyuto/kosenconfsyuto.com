import Vapor
import Yams

func routes(_ app: Application) throws {
    app.get(":file") { req -> EventLoopFuture<Response> in
        let file = req.parameters.get("file") ?? "index.html"
        let pagesDirectory = app.directory.workingDirectory + "pages/"
        let filePath = pagesDirectory + file
        
        return req.fileio.collectFile(at: filePath).flatMapThrowing { buffer -> (String, [String: Any]) in
            guard var content = buffer.getString(at: 0, length: buffer.readableBytes) else {
                throw Abort(.internalServerError, reason: "Failed to read file content")
            }
            
            var yamlData: [String: Any] = [:]

            // Extract YAML front matter
            if content.hasPrefix("---") {
                let endIndex = content.index(content.startIndex, offsetBy: 3)
                if let range = content[endIndex...].range(of: "---") {
                    let yamlContent = String(content[endIndex..<range.lowerBound]).trimmingCharacters(in: .whitespacesAndNewlines)
                    content = String(content[range.upperBound...])
                    
                    // Parse the YAML content
                    do {
                        if let yaml = try Yams.load(yaml: yamlContent) as? [String: Any] {
                            yamlData = yaml
                        }
                    } catch {
                        throw Abort(.internalServerError, reason: "Failed to parse YAML front matter")
                    }
                }
            }
            
            return (content, yamlData)
        }.flatMap { (content, yamlData) -> EventLoopFuture<(String, [String: Any])> in
            guard let commonParts = yamlData["CommonParts"] as? String else {
                return req.eventLoop.future((content, yamlData))
            }
            let partFiles = commonParts.split(separator: "|").map { String($0).trimmingCharacters(in: .whitespaces) }.reversed()
            return applyCommonParts(req: req, parts: Array(partFiles), content: content).map { newContent in
                (newContent, yamlData)
            }
        }.flatMapThrowing { (html, yamlData) in
            var finalHtml = html
            if let title = yamlData["title"] as? String {
                finalHtml = finalHtml.replacingOccurrences(of: "{{title}}", with: title)
            }
            let response = Response(status: .ok, headers: HTTPHeaders([("Content-Type", "text/html")]))
            response.body = .init(string: finalHtml)
            return response
        }
    }

    // Other routes...
    try app.register(collection: TodoController())
}

func applyCommonParts(req: Request, parts: [String], content: String) -> EventLoopFuture<String> {
    let part = parts.first
    let remainingParts = Array(parts.dropFirst())
    let commonPartsPath = req.application.directory.workingDirectory + "includes/\(part!).html"
    
    return req.fileio.collectFile(at: commonPartsPath).flatMapThrowing { buffer -> String in
        guard let commonPartsContent = buffer.getString(at: 0, length: buffer.readableBytes) else {
            throw Abort(.internalServerError, reason: "Failed to read common parts content")
        }
        
        // Replace {{contents}} with the content
        let html = commonPartsContent.replacingOccurrences(of: "{{contents}}", with: content)
        return html
    }.flatMap { newContent in
        if remainingParts.isEmpty {
            return req.eventLoop.future(newContent)
        } else {
            return applyCommonParts(req: req, parts: remainingParts, content: newContent)
        }
    }
}
