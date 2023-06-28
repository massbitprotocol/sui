// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

module sui::transfer {

    use sui::object::{Self, ID};
    use sui::prover;

    #[test_only]
    friend sui::test_scenario;

    /// This represents an obligation to `receive` an object of type `T` since
    /// there is no key/store/drop abilities. Internals of this struct are
    /// opaque outside this module.
    struct Receiving<phantom T: key> has drop {
        id: ID,
        version: u64,
    }

    /// Shared an object that was previously created. Shared objects must currently
    /// be constructed in the transaction they are created.
    const ESharedNonNewObject: u64 = 0;

    /// Transfer ownership of `obj` to `recipient`. `obj` must have the `key` attribute,
    /// which (in turn) ensures that `obj` has a globally unique ID. Note that if the recipient
    /// address represents an object ID, the `obj` sent will be inaccessible after the transfer
    /// (though they will be retrievable at a future date once new features are added).
    /// This function has custom rules performed by the Sui Move bytecode verifier that ensures
    /// that `T` is an object defined in the module where `transfer` is invoked. Use
    /// `public_transfer` to transfer an object with `store` outside of its module.
    public fun transfer<T: key>(obj: T, recipient: address) {
        transfer_impl(obj, recipient)
    }

    /// Transfer ownership of `obj` to `recipient`. `obj` must have the `key` attribute,
    /// which (in turn) ensures that `obj` has a globally unique ID. Note that if the recipient
    /// address represents an object ID, the `obj` sent will be inaccessible after the transfer
    /// (though they will be retrievable at a future date once new features are added).
    /// The object must have `store` to be transferred outside of its module.
    public fun public_transfer<T: key + store>(obj: T, recipient: address) {
        transfer_impl(obj, recipient)
    }

    /// Freeze `obj`. After freezing `obj` becomes immutable and can no longer be transferred or
    /// mutated.
    /// This function has custom rules performed by the Sui Move bytecode verifier that ensures
    /// that `T` is an object defined in the module where `freeze_object` is invoked. Use
    /// `public_freeze_object` to freeze an object with `store` outside of its module.
    public fun freeze_object<T: key>(obj: T) {
        freeze_object_impl(obj)
    }

    /// Freeze `obj`. After freezing `obj` becomes immutable and can no longer be transferred or
    /// mutated.
    /// The object must have `store` to be frozen outside of its module.
    public fun public_freeze_object<T: key + store>(obj: T) {
        freeze_object_impl(obj)
    }

    /// Turn the given object into a mutable shared object that everyone can access and mutate.
    /// This is irreversible, i.e. once an object is shared, it will stay shared forever.
    /// Aborts with `ESharedNonNewObject` of the object being shared was not created in this
    /// transaction. This restriction may be relaxed in the future.
    /// This function has custom rules performed by the Sui Move bytecode verifier that ensures
    /// that `T` is an object defined in the module where `share_object` is invoked. Use
    /// `public_share_object` to share an object with `store` outside of its module.
    public fun share_object<T: key>(obj: T) {
        share_object_impl(obj)
    }

    /// Turn the given object into a mutable shared object that everyone can access and mutate.
    /// This is irreversible, i.e. once an object is shared, it will stay shared forever.
    /// Aborts with `ESharedNonNewObject` of the object being shared was not created in this
    /// transaction. This restriction may be relaxed in the future.
    /// The object must have `store` to be shared outside of its module.
    public fun public_share_object<T: key + store>(obj: T) {
        share_object_impl(obj)
    }

    /// Given mutable (i.e., locked) access to the `parent` and a `Receiving`
    /// object referencing an object owned by `parent` discharge the `Receiving` obligation
    /// and return the corresponding owned object.
    public fun receive<T: key>(parent: &mut object::UID, to_receive: Receiving<T>): T {
        let Receiving {
            id,
            version,
        } = to_receive;
        receive_impl(object::uid_to_address(parent), id, version)
    }

    public(friend) native fun freeze_object_impl<T: key>(obj: T);

    spec freeze_object_impl {
        pragma opaque;
        // never aborts as it requires object by-value and:
        // - it's OK to freeze whether object is fresh or owned
        // - shared or immutable object cannot be passed by value
        aborts_if [abstract] false;
        modifies [abstract] global<object::Ownership>(object::id(obj).bytes);
        ensures [abstract] exists<object::Ownership>(object::id(obj).bytes);
        ensures [abstract] global<object::Ownership>(object::id(obj).bytes).status == prover::IMMUTABLE;
    }

    public(friend) native fun share_object_impl<T: key>(obj: T);

    spec share_object_impl {
        pragma opaque;
        aborts_if [abstract] sui::prover::owned(obj);
        modifies [abstract] global<object::Ownership>(object::id(obj).bytes);
        ensures [abstract] exists<object::Ownership>(object::id(obj).bytes);
        ensures [abstract] global<object::Ownership>(object::id(obj).bytes).status == prover::SHARED;
    }


    public(friend) native fun transfer_impl<T: key>(obj: T, recipient: address);

    spec transfer_impl {
        pragma opaque;
        // never aborts as it requires object by-value and:
        // - it's OK to transfer whether object is fresh or already owned
        // - shared or immutable object cannot be passed by value
        aborts_if [abstract] false;
        modifies [abstract] global<object::Ownership>(object::id(obj).bytes);
        ensures [abstract] exists<object::Ownership>(object::id(obj).bytes);
        ensures [abstract] global<object::Ownership>(object::id(obj).bytes).owner == recipient;
        ensures [abstract] global<object::Ownership>(object::id(obj).bytes).status == prover::OWNED;
    }

    native fun receive_impl<T: key>(parent: address, to_receive: object::ID, version: u64): T;
}
