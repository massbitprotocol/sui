// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
	useGetOwnedObjects,
	hasDisplayData,
	useGetKioskContents,
	isKioskOwnerToken,
} from '@mysten/core';
import { type SuiObjectData, type SuiAddress } from '@mysten/sui.js';

import useAppSelector from './useAppSelector';

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

	const { apiEnv } = useAppSelector((state) => state.app);
	const disableOriginByteKiosk = apiEnv !== 'mainnet';

	const { data: kioskData, isLoading: areKioskContentsLoading } = useGetKioskContents(
		address,
		disableOriginByteKiosk,
	);

	const kioskOwnerTokens = data?.pages
		.flatMap((page) => page.data)
		.filter(isKioskOwnerToken)
		.map(({ data }) => data as SuiObjectData);

	const kiosks = Object.entries(kioskData?.kiosks ?? {}).map(([kioskId, kiosk]) => kiosk);

	const nfts = {
		kioskOwnerTokens,
		kiosks,
		ownedObjects:
			data?.pages
				.flatMap((page) => page.data)
				.filter((object) => !isKioskOwnerToken(object))
				.filter(hasDisplayData)
				.map(({ data }) => data as SuiObjectData) || [],
	};

	return {
		data: nfts,
		isInitialLoading,
		hasNextPage,
		isFetchingNextPage,
		fetchNextPage,
		isLoading: isLoading || areKioskContentsLoading,
		isError: isError,
		error,
	};
}
