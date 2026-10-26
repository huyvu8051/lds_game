//
//  ViewController+CoreMotion.swift
//
//  Gyroscope aiming. The phone's own turn is read as angular velocity and
//  handed to Rust as two angles, which native_metal_probe_gyro_moved feeds
//  in as pointer travel — so it arrives on the same path as a mouse and
//  inherits the aim multiplier and the look lag rather than fighting them.
//
//  The sensor only runs while the setting is on: CoreMotion delivery costs
//  a queue and a wakeup per sample, which a player who left the switch
//  alone should not be paying.
//

import UIKit
import CoreMotion

@available(iOS 14.0, *)
final class GyroInput {
    private let motion = CMMotionManager()
    private var running = false

    init() {
        // 60Hz: the look path samples once a frame, so asking the sensor for
        // more only buys samples that are averaged away.
        motion.deviceMotionUpdateInterval = 1.0 / 60.0
    }

    /// Starts or stops the sensor to match the setting, then drains whatever
    /// has arrived since the last frame.
    func update(probe: OpaquePointer, orientation: UIInterfaceOrientation, deltaTime: Float) {
        let wanted = native_metal_probe_get_gyro_enabled(probe)
        if wanted != running {
            if wanted && motion.isDeviceMotionAvailable {
                // xArbitraryZVertical keeps Z on gravity, which is what makes
                // a turn about the vertical read as a turn rather than as a
                // mixture of two axes once the phone is tilted.
                motion.startDeviceMotionUpdates(using: .xArbitraryZVertical)
                running = true
            } else if !wanted {
                motion.stopDeviceMotionUpdates()
                running = false
            }
        }
        guard running, let sample = motion.deviceMotion else { return }

        // rotationRate is in the device's own frame, which is defined in
        // portrait: +X out the right edge, +Y out the top, +Z out of the
        // screen. Which of those is the world's vertical depends on how the
        // phone is being held, so the interface orientation picks the pair.
        let rate = sample.rotationRate
        let yaw: Double
        let pitch: Double
        switch orientation {
        case .landscapeRight:
            // Home edge to the right, so the portrait-right edge points up.
            yaw = -rate.x
            pitch = -rate.y
        case .landscapeLeft:
            yaw = rate.x
            pitch = rate.y
        default:
            yaw = -rate.y
            pitch = rate.x
        }
        // Rate times the frame is the angle turned, which is what the Rust
        // side expects: radians since the last sample.
        native_metal_probe_gyro_moved(probe, Float(yaw) * deltaTime, Float(pitch) * deltaTime)
    }

    deinit {
        motion.stopDeviceMotionUpdates()
    }
}
