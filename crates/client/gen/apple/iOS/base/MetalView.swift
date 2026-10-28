//
//  MetalView.swift
//  bevy_in_iOS
//
//  Created by Jinlei Li on 2022/12/20.
//

import UIKit
import Foundation

class MetalView: UIView {
    override class var layerClass: AnyClass {
        return CAMetalLayer.self
    }
    
    override func awakeFromNib() {
        super.awakeFromNib()
        configLayer()
        // Opaque, not clear. A clear background told the compositor this layer
        // might have something showing through it, so backboardd had to blend
        // the whole drawable every frame rather than copy it. The Metal clear
        // colour covers every pixel at full alpha, so there is nothing behind
        // it to see. The renderer sets CAMetalLayer.isOpaque to match.
        self.isOpaque = true
        self.layer.isOpaque = true
        self.layer.backgroundColor = UIColor.black.cgColor
        // UIView.isMultipleTouchEnabled defaults to false — UIKit then only
        // ever tracks ONE touch at a time for this view, silently dropping
        // every other simultaneous touch before touchesBegan ever sees it.
        // This alone could fully explain "combo touches don't work": the
        // second finger's events may never have reached Swift at all, let
        // alone Rust/Bevy.
        self.isMultipleTouchEnabled = true
    }
    
    private func configLayer() {
        guard let layer = self.layer as? CAMetalLayer else {
            return
        }
        layer.presentsWithTransaction = false
        layer.framebufferOnly = true
        // nativeScale is real physical pixel scale
        // https://tomisacat.xyz/tech/2017/06/17/scale-nativescale-contentsscale.html
        self.contentScaleFactor = UIScreen.main.nativeScale
    }
}

