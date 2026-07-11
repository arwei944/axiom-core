use axiom_macros::capability;

#[capability(dim = "layer", version = "1.0.0", layer = "bad_layer")]
struct BadLayerCapability;
