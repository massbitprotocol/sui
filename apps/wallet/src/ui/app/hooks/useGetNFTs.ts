// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useGetOwnedObjects } from '@mysten/core';
import { type SuiObjectData, type SuiAddress } from '@mysten/sui.js';

export function useGetNFTs(address?: SuiAddress | null) {
	const {
		data,
		isLoading,
		error,
		isError,
		isFetchingNextPage,
		hasNextPage,
		fetchNextPage,
		isInitialLoading,
	} = useGetOwnedObjects(
		address,
		{
			MatchNone: [{ StructType: '0x2::coin::Coin' }],
		},
		50,
	);

	const ownedAssets =
		data?.pages.flatMap((page) => page.data).map(({ data }) => data as SuiObjectData) ?? [];

	// const flat = data?.pages.flatMap((page) => page.data) ?? [];
	// const { kioskOwnerTokens, ownedObjects } = flat.reduce(
	// 	(acc, curr) => {
	// 		if (!curr.data) return acc;

	// 		if (isKioskOwnerToken(curr)) {
	// 			acc.kioskOwnerTokens.push(curr.data);
	// 			return acc;
	// 		}
	// 		acc.ownedObjects.push(curr.data);
	// 		return acc;
	// 	},
	// 	{ kioskOwnerTokens: [], ownedObjects: [] } as OwnedAssets,
	// );

	// const nfts = {
	// 	kioskOwnerTokens,
	// 	ownedObjects,
	// };

	return {
		data: ownedAssets,
		isInitialLoading,
		hasNextPage,
		isFetchingNextPage,
		fetchNextPage,
		isLoading: isLoading,
		isError: isError,
		error,
	};
}
