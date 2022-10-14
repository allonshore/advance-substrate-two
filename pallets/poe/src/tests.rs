use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, BoundedVec};

#[test]
fn test_create_claim_works() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		assert_ok!(PoeModule::create_claim(Origin::signed(1), claim.clone()));
		let bounded_claim =
			BoundedVec::<u8, <Test as Config>::MaxClaimLength>::try_from(claim.clone()).unwrap();
		assert_eq!(
			Proofs::<Test>::get(&bounded_claim),
			Some((1, frame_system::Pallet::<Test>::block_number()))
		);
	});
}

#[test]
fn test_create_claim_failed_when_claim_already_exists() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		let _ = PoeModule::create_claim(Origin::signed(1), claim.clone());
		assert_noop!(
			PoeModule::create_claim(Origin::signed(1), claim.clone()),
			Error::<Test>::ProofAlreadyExist
		);
	});
}
#[test]
fn test_claim_too_long() {
	new_test_ext().execute_with(|| {
		let claim = vec![0; 9999];

		assert_noop!(
			PoeModule::create_claim(Origin::signed(1), claim),
			Error::<Test>::ClaimTooLong
		);
	});
}

#[test]
fn test_claim_not_exist() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		assert_ok!(PoeModule::create_claim(Origin::signed(1), claim.clone()));

		assert_ok!(PoeModule::revoke_claim(Origin::signed(1), claim));
	});
}

#[test]
fn test_not_claim_owner() {
	new_test_ext().execute_with(|| {
		let claim = vec![0, 1];
		assert_ok!(PoeModule::create_claim(Origin::signed(1), claim.clone()));
		let bounded_claim =
			BoundedVec::<u8, <Test as Config>::MaxClaimLength>::try_from(claim.clone()).unwrap();

		assert_ok!(PoeModule::transfer_claim(Origin::signed(1), claim.clone(), 2));
		assert_eq!(
			Proofs::<Test>::get(&bounded_claim),
			Some((2, frame_system::Pallet::<Test>::block_number()))
		);
	})
}
