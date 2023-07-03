// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { isKioskOwnerToken, useGetObject } from '@mysten/core';
import { formatAddress } from '@mysten/sui.js';
import { cva, cx } from 'class-variance-authority';

import { useResolveVideo } from '../../hooks/useResolveVideo';
import { Kiosk } from './Kiosk';
import { Heading } from '_app/shared/heading';
import Loading from '_components/loading';
import { NftImage, type NftImageProps } from '_components/nft-display/NftImage';
import { useGetNFTMeta, useFileExtensionType } from '_hooks';

import type { VariantProps } from 'class-variance-authority';

const nftDisplayCardStyles = cva('flex flex-nowrap items-center h-full', {
	variants: {
		animateHover: {
			true: 'group',
		},
		wideView: {
			true: 'bg-gray-40 p-2.5 rounded-lg gap-2.5 flex-row-reverse justify-between',
			false: 'flex-col',
		},
	},
	defaultVariants: {
		wideView: false,
	},
});

export interface NFTsProps extends VariantProps<typeof nftDisplayCardStyles> {
	objectId: string;
	showLabel?: boolean;
	size: NftImageProps['size'];
	borderRadius?: NftImageProps['borderRadius'];
	playable?: boolean;
}

export function NFTDisplayCard({
	objectId,
	showLabel,
	size,
	wideView,
	animateHover,
	borderRadius = 'md',
	playable,
}: NFTsProps) {
	const { data: objectData } = useGetObject(objectId);

	const isOwnerToken = isKioskOwnerToken(objectData);

	const { data: nftMeta, isLoading } = useGetNFTMeta(objectId);
	const nftName = nftMeta?.name || formatAddress(objectId);
	const nftImageUrl = nftMeta?.imageUrl || '';
	const video = useResolveVideo(objectData);
	const fileExtensionType = useFileExtensionType(nftImageUrl);

	return (
		<div className={nftDisplayCardStyles({ animateHover, wideView })}>
			<Loading loading={isLoading}>{isOwnerToken ? <Kiosk object={objectData} /> : null}</Loading>
		</div>
	);
}
