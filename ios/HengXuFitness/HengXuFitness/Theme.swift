import SwiftUI

enum HXTheme {
    static let deep = Color(red: 0.07, green: 0.14, blue: 0.11)
    static let green = Color(red: 0.16, green: 0.48, blue: 0.36)
    static let mint = Color(red: 0.89, green: 0.95, blue: 0.91)
    static let lime = Color(red: 0.82, green: 0.91, blue: 0.63)
    static let canvas = Color(red: 0.96, green: 0.97, blue: 0.95)
}

struct HXCardModifier: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding(16)
            .background(.white, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 20, style: .continuous).stroke(Color.black.opacity(0.06)))
    }
}

extension View {
    func hxCard() -> some View { modifier(HXCardModifier()) }
}
